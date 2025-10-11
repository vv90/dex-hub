use alloy::{primitives::FixedBytes, providers::fillers::RecommendedFillers, sol_types::SolEvent};
use anyhow::{Result, anyhow};
use futures_util::{Stream, StreamExt, TryStreamExt};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::{stream, sync::mpsc, task::JoinHandle};

use crate::{
    Blockchain,
    blockchain::{BlockNumber, BlockchainNetwork},
    evm_network, pancakeswap_internal as pancakeswap,
    pool_id::PoolId,
    rpc,
    state_manager::{
        event::{Event, EventId, EventInfo},
        pool_reserves_calls::ReservesCallData,
        protocol_addresses::ProtocolAddresses,
    },
    tokens::{TokenAddress, TokenInfo},
    uniswap_internal as uniswap,
    virtual_reserves::VirtualReserves,
};

const TOPICS: [FixedBytes<32>; 10] = [
    uniswap::v2::contract::Pair::Mint::SIGNATURE_HASH,
    uniswap::v2::contract::Pair::Burn::SIGNATURE_HASH,
    uniswap::v2::contract::Pair::Swap::SIGNATURE_HASH,
    uniswap::v2::contract::Pair::Sync::SIGNATURE_HASH,
    uniswap::v3::contract::Pool::Mint::SIGNATURE_HASH,
    uniswap::v3::contract::Pool::Burn::SIGNATURE_HASH,
    uniswap::v3::contract::Pool::Swap::SIGNATURE_HASH,
    uniswap::v4::contract::PoolManager::ModifyLiquidity::SIGNATURE_HASH,
    uniswap::v4::contract::PoolManager::Swap::SIGNATURE_HASH,
    uniswap::v4::contract::PoolManager::Donate::SIGNATURE_HASH,
];

pub struct StateManager {
    protocol_addresses_ethereum: ProtocolAddresses<evm_network::Ethereum>,
    protocol_addresses_bsc: ProtocolAddresses<evm_network::BSC>,
    protocol_addresses_arbitrum: ProtocolAddresses<evm_network::Arbitrum>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            protocol_addresses_ethereum: ProtocolAddresses::new(),
            protocol_addresses_bsc: ProtocolAddresses::new(),
            protocol_addresses_arbitrum: ProtocolAddresses::new(),
        }
    }

    fn with_ethereum_protocol_address(
        self,
        update_fn: impl Fn(
            ProtocolAddresses<evm_network::Ethereum>,
        ) -> ProtocolAddresses<evm_network::Ethereum>,
    ) -> Self {
        Self {
            protocol_addresses_ethereum: update_fn(self.protocol_addresses_ethereum),
            protocol_addresses_bsc: self.protocol_addresses_bsc,
            protocol_addresses_arbitrum: self.protocol_addresses_arbitrum,
        }
    }

    fn with_bsc_protocol_address(
        self,
        update_fn: impl Fn(ProtocolAddresses<evm_network::BSC>) -> ProtocolAddresses<evm_network::BSC>,
    ) -> Self {
        Self {
            protocol_addresses_ethereum: self.protocol_addresses_ethereum,
            protocol_addresses_bsc: update_fn(self.protocol_addresses_bsc),
            protocol_addresses_arbitrum: self.protocol_addresses_arbitrum,
        }
    }

    fn with_arbitrum_protocol_address(
        self,
        update_fn: impl Fn(
            ProtocolAddresses<evm_network::Arbitrum>,
        ) -> ProtocolAddresses<evm_network::Arbitrum>,
    ) -> Self {
        Self {
            protocol_addresses_ethereum: self.protocol_addresses_ethereum,
            protocol_addresses_bsc: self.protocol_addresses_bsc,
            protocol_addresses_arbitrum: update_fn(self.protocol_addresses_arbitrum),
        }
    }

    pub fn from_pools(pools: &HashSet<PoolId>) -> Self {
        pools
            .into_iter()
            .fold(Self::new(), |events_manager, pool_id| match pool_id {
                PoolId::UniswapV2(uniswap::v2::pool::PoolAddress(
                    address,
                    Blockchain::Ethereum,
                )) => events_manager.with_ethereum_protocol_address(|pa| {
                    pa.with_v2_address(*address, EventId::UniswapV2)
                }),
                PoolId::UniswapV2(uniswap::v2::pool::PoolAddress(address, Blockchain::BSC)) => {
                    events_manager.with_bsc_protocol_address(|pa| {
                        pa.with_v2_address(*address, EventId::UniswapV2)
                    })
                }
                PoolId::UniswapV2(uniswap::v2::pool::PoolAddress(
                    address,
                    Blockchain::Arbitrum,
                )) => events_manager.with_arbitrum_protocol_address(|pa| {
                    pa.with_v2_address(*address, EventId::UniswapV2)
                }),

                PoolId::UniswapV3(uniswap::v3::pool::PoolAddress(
                    address,
                    Blockchain::Ethereum,
                )) => events_manager.with_ethereum_protocol_address(|pa| {
                    pa.with_v3_address(*address, EventId::UniswapV3)
                }),
                PoolId::UniswapV3(uniswap::v3::pool::PoolAddress(address, Blockchain::BSC)) => {
                    events_manager.with_bsc_protocol_address(|pa| {
                        pa.with_v3_address(*address, EventId::UniswapV3)
                    })
                }
                PoolId::UniswapV3(uniswap::v3::pool::PoolAddress(
                    address,
                    Blockchain::Arbitrum,
                )) => events_manager.with_arbitrum_protocol_address(|pa| {
                    pa.with_v3_address(*address, EventId::UniswapV3)
                }),

                PoolId::UniswapV4(uniswap::v4::pool::PoolId(id, Blockchain::Ethereum)) => {
                    events_manager
                        .with_ethereum_protocol_address(|pa| pa.with_v4_id(*id, EventId::UniswapV4))
                }
                PoolId::UniswapV4(uniswap::v4::pool::PoolId(id, Blockchain::BSC)) => events_manager
                    .with_bsc_protocol_address(|pa| pa.with_v4_id(*id, EventId::UniswapV4)),
                PoolId::UniswapV4(uniswap::v4::pool::PoolId(id, Blockchain::Arbitrum)) => {
                    events_manager
                        .with_arbitrum_protocol_address(|pa| pa.with_v4_id(*id, EventId::UniswapV4))
                }

                PoolId::PancakeSwap(pancakeswap::v3::pool::PoolAddress(
                    address,
                    Blockchain::Ethereum,
                )) => events_manager.with_ethereum_protocol_address(|pa| {
                    pa.with_v3_address(*address, EventId::PancakeSwap)
                }),
                PoolId::PancakeSwap(pancakeswap::v3::pool::PoolAddress(
                    address,
                    Blockchain::BSC,
                )) => events_manager.with_bsc_protocol_address(|pa| {
                    pa.with_v3_address(*address, EventId::PancakeSwap)
                }),
                PoolId::PancakeSwap(pancakeswap::v3::pool::PoolAddress(
                    address,
                    Blockchain::Arbitrum,
                )) => events_manager.with_arbitrum_protocol_address(|pa| {
                    pa.with_v3_address(*address, EventId::PancakeSwap)
                }),
            })
    }

    pub async fn subscribe_reserves(
        self,
        sender: mpsc::UnboundedSender<Vec<(PoolId, VirtualReserves)>>,
        tokens: Arc<HashMap<TokenAddress, TokenInfo>>,
        uniswap_v2_pools: Arc<HashMap<uniswap::v2::pool::PoolAddress, uniswap::v2::pool::PoolInfo>>,
        uniswap_v3_pools: Arc<HashMap<uniswap::v3::pool::PoolAddress, uniswap::v3::pool::PoolInfo>>,
        uniswap_v4_pools: Arc<HashMap<uniswap::v4::pool::PoolId, uniswap::v4::pool::PoolInfo>>,
        pancake_swap_pools: Arc<
            HashMap<pancakeswap::v3::pool::PoolAddress, pancakeswap::v3::pool::PoolInfo>,
        >,
    ) -> Result<(JoinHandle<Result<()>>, HashMap<PoolId, VirtualReserves>)> {
        let (ethereum_join_handle, initial_reserves_ethereum) = subscribe_reserve_updates(
            sender.clone(),
            Arc::new(self.protocol_addresses_ethereum),
            tokens.clone(),
            uniswap_v2_pools.clone(),
            uniswap_v3_pools.clone(),
            uniswap_v4_pools.clone(),
            pancake_swap_pools.clone(),
        )
        .await?;
        let (bsc_join_handle, initial_reserves_bsc) = subscribe_reserve_updates(
            sender.clone(),
            Arc::new(self.protocol_addresses_bsc),
            tokens.clone(),
            uniswap_v2_pools.clone(),
            uniswap_v3_pools.clone(),
            uniswap_v4_pools.clone(),
            pancake_swap_pools.clone(),
        )
        .await?;
        let (arbitrum_join_handle, initial_reserves_arbitrum) = subscribe_reserve_updates(
            sender.clone(),
            Arc::new(self.protocol_addresses_arbitrum),
            tokens.clone(),
            uniswap_v2_pools.clone(),
            uniswap_v3_pools.clone(),
            uniswap_v4_pools.clone(),
            pancake_swap_pools.clone(),
        )
        .await?;

        let combined_join_handle = tokio::spawn(async move {
            tokio::try_join!(ethereum_join_handle, bsc_join_handle, arbitrum_join_handle)
                .map_err(|_| anyhow!("Failed to join handles"))
                .and_then(|(a, b, c)| a.and(b).and(c))
        });
        let combined_initial_reserves = initial_reserves_ethereum
            .into_iter()
            .chain(initial_reserves_bsc.into_iter())
            .chain(initial_reserves_arbitrum.into_iter())
            .collect::<HashMap<PoolId, VirtualReserves>>();
        Ok((combined_join_handle, combined_initial_reserves))
    }
}

async fn subscribe_reserve_updates<B: BlockchainNetwork + RecommendedFillers>(
    sender: mpsc::UnboundedSender<Vec<(PoolId, VirtualReserves)>>,
    protocol_addresses: Arc<ProtocolAddresses<B>>,
    tokens: Arc<HashMap<TokenAddress, TokenInfo>>,
    uniswap_v2_pools: Arc<HashMap<uniswap::v2::pool::PoolAddress, uniswap::v2::pool::PoolInfo>>,
    uniswap_v3_pools: Arc<HashMap<uniswap::v3::pool::PoolAddress, uniswap::v3::pool::PoolInfo>>,
    uniswap_v4_pools: Arc<HashMap<uniswap::v4::pool::PoolId, uniswap::v4::pool::PoolInfo>>,
    pancake_swap_pools: Arc<
        HashMap<pancakeswap::v3::pool::PoolAddress, pancakeswap::v3::pool::PoolInfo>,
    >,
) -> Result<(JoinHandle<Result<()>>, Vec<(PoolId, VirtualReserves)>)> {
    println!("Subscribing to pool updates on {}", {
        B::BLOCKCHAIN.name()
    });
    let (events_sender, mut events_receiver) = mpsc::unbounded_channel::<Event<B>>();

    let protocol_addresses_clone = protocol_addresses.clone();

    let rpc_client = rpc::client::init_client::<B>().await?;

    rpc::client::subscribe_topics::<Event<B>, B>(
        events_sender.clone(),
        HashSet::from(TOPICS),
        move |log| protocol_addresses_clone.try_lookup(log).unwrap(), // TODO: Handle event parsing errors
    )
    .await?;

    let block_number = rpc_client.get_block_number().await?;
    let initial_calls = protocol_addresses.initial_calls(
        tokens.as_ref(),
        uniswap_v2_pools.as_ref(),
        uniswap_v3_pools.as_ref(),
        uniswap_v4_pools.as_ref(),
        pancake_swap_pools.as_ref(),
    )?;

    let initial_reserves = futures_util::stream::iter(initial_calls.chunks(1000))
        .map(Ok::<&[ReservesCallData<B>], anyhow::Error>)
        .try_fold(
            Vec::<(PoolId, VirtualReserves)>::new(),
            async |mut combined_reserves, chunk| {
                let reserves = rpc_client
                    .get_multicall(chunk, block_number)
                    .await?
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()?;
                combined_reserves.extend(reserves);
                Ok(combined_reserves)
            },
        )
        .await?;

    let handle = tokio::spawn(async move {
        loop {
            let mut buffer = Vec::new();
            let received_count = events_receiver.recv_many(&mut buffer, 1000).await;

            println!("{} received", received_count);

            if let Some((head, tail)) = buffer.split_first() {
                let (events_map, block_number) = tail.into_iter().fold(
                    (
                        HashMap::<EventId, &EventInfo<B>>::from([(head.id, &head.info)]),
                        head.info.block_number,
                    ),
                    |(mut events, latest_block_number), event| {
                        events
                            .entry(event.id)
                            .and_modify(|existing| {
                                if existing.block_number.value() < event.info.block_number.value() {
                                    *existing = &event.info;
                                }
                            })
                            .or_insert_with(|| &event.info);
                        (
                            events,
                            BlockNumber::pick_latest(latest_block_number, event.info.block_number),
                        )
                    },
                );

                let calls = events_map
                    .into_iter()
                    .map(|(event_id, _)| {
                        event_id.into_call_data(
                            tokens.as_ref(),
                            uniswap_v2_pools.as_ref(),
                            uniswap_v3_pools.as_ref(),
                            uniswap_v4_pools.as_ref(),
                            pancake_swap_pools.as_ref(),
                        )
                    })
                    .collect::<Result<Vec<_>>>()?;

                let updated_reserves = rpc_client
                    .get_multicall(&calls, block_number)
                    .await?
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()?;

                sender.send(updated_reserves)?;
            } else {
                println!("Logs subscription terminated");
                break;
            }
        }
        Ok(())
    });

    Ok((handle, initial_reserves))
}
