use alloy::{primitives::FixedBytes, providers::fillers::RecommendedFillers, sol_types::SolEvent};
use anyhow::Result;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::mpsc::{self};

use crate::{
    Blockchain, DexInfo,
    blockchain::{BlockNumber, BlockchainNetwork},
    evm_network, pancakeswap_internal as pancakeswap,
    pool_id::PoolId,
    rpc::{
        self,
        client::{NetworkProvider, RpcClient},
    },
    state_manager::{
        event::{Event, EventId},
        protocol_addresses::ProtocolAddresses,
    },
    uniswap_internal as uniswap,
    virtual_reserves::VirtualReserves,
};

pub const TOPICS: [FixedBytes<32>; 10] = [
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

struct BlockchainStateManager<B: BlockchainNetwork + RecommendedFillers> {
    events_receiver: mpsc::UnboundedReceiver<Vec<Event<B>>>,
    dex_info: Arc<DexInfo>,
    rpc_client: RpcClient<B, NetworkProvider<B>>,
}

impl<B: BlockchainNetwork + RecommendedFillers> BlockchainStateManager<B> {
    pub async fn init(
        protocol_addresses: ProtocolAddresses<B>,
        dex_info: Arc<DexInfo>,
    ) -> Result<(Self, Vec<(PoolId, VirtualReserves)>)> {
        let (event_sender, mut events_receiver) = mpsc::unbounded_channel::<Vec<Event<B>>>();
        let rpc_client = rpc::client::init_client().await?;

        let reserve_calls = protocol_addresses.initial_calls(dex_info.as_ref())?;

        tokio::spawn(async move {
            // logs come in bursts as new blocks are minted
            let mut log_receiver = rpc::client::subscribe_logs::<B>(HashSet::from(TOPICS)).await?;

            let mut buffer = Vec::new();

            loop {
                log_receiver.recv_many(&mut buffer, usize::MAX).await;
                println!("Received {} logs on {}", buffer.len(), B::BLOCKCHAIN.name());
                let events = buffer
                    .drain(..)
                    .filter_map(|log| protocol_addresses.try_lookup(log).unwrap()) // TODO: Handle event parsing errors
                    .collect::<Vec<_>>();
                if events.is_empty() {
                    continue;
                }
                if let Err(err) = event_sender.send(events) {
                    println!("BlockchainStateManager: Failed to send events: {}", err);
                    break;
                };
            }

            anyhow::Ok(())
        });

        let first_events_batch = events_receiver.recv().await;

        let block_number = first_events_batch
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Failed to receive first events batch from {}",
                    B::BLOCKCHAIN.name()
                )
            })
            .and_then(|events| {
                events
                    .into_iter()
                    .map(|event| event.info.block_number)
                    .reduce(BlockNumber::pick_latest)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Failed to pick latest block number from {} events: Empty events batch",
                            B::BLOCKCHAIN.name()
                        )
                    })
            })?;

        println!("Requesting initial reserves on {}", B::BLOCKCHAIN.name());
        let reserves = rpc_client
            .get_multicall_chunked(&reserve_calls, block_number, 1000)
            .await?;

        let reserves = reserves.into_iter().collect::<Result<Vec<_>, _>>()?;

        Ok((
            Self {
                events_receiver,
                dex_info,
                rpc_client,
            },
            reserves,
        ))
    }

    pub async fn get_updated_reserves(mut self) -> Result<(Self, Vec<(PoolId, VirtualReserves)>)> {
        let size = self.events_receiver.len();
        let mut buffer = Vec::with_capacity(size);
        let _ = self.events_receiver.recv_many(&mut buffer, size).await;

        let mut buffer_iter = buffer.into_iter().flatten();

        match buffer_iter.next() {
            Some(head) => {
                let (events_set, block_number) = buffer_iter.fold(
                    (HashSet::<EventId>::new(), head.info.block_number),
                    |(mut events_set, block_number), event| {
                        events_set.insert(event.id);
                        (
                            events_set,
                            BlockNumber::pick_latest(block_number, event.info.block_number),
                        )
                    },
                );
                let calls = events_set
                    .into_iter()
                    .map(|event_id| event_id.into_call_data::<B>(self.dex_info.as_ref()))
                    .collect::<Result<Vec<_>>>()?;

                let reserves = self
                    .rpc_client
                    .get_multicall_chunked(&calls, block_number, 1000)
                    .await?;

                let reserves = reserves.into_iter().collect::<Result<_, _>>()?;

                Ok((self, reserves))
            }
            None => Ok((self, vec![])),
        }
    }
}

struct StateManagerBuilder {
    protocol_addresses_ethereum: ProtocolAddresses<evm_network::Ethereum>,
    protocol_addresses_bsc: ProtocolAddresses<evm_network::BSC>,
    protocol_addresses_arbitrum: ProtocolAddresses<evm_network::Arbitrum>,
}

impl StateManagerBuilder {
    fn new() -> Self {
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
            ..self
        }
    }

    fn with_bsc_protocol_address(
        self,
        update_fn: impl Fn(ProtocolAddresses<evm_network::BSC>) -> ProtocolAddresses<evm_network::BSC>,
    ) -> Self {
        Self {
            protocol_addresses_bsc: update_fn(self.protocol_addresses_bsc),
            ..self
        }
    }

    fn with_arbitrum_protocol_address(
        self,
        update_fn: impl Fn(
            ProtocolAddresses<evm_network::Arbitrum>,
        ) -> ProtocolAddresses<evm_network::Arbitrum>,
    ) -> Self {
        Self {
            protocol_addresses_arbitrum: update_fn(self.protocol_addresses_arbitrum),
            ..self
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
}

pub struct StateManager {
    state_manager_ethereum: BlockchainStateManager<evm_network::Ethereum>,
    state_manager_bsc: BlockchainStateManager<evm_network::BSC>,
    state_manager_arbitrum: BlockchainStateManager<evm_network::Arbitrum>,
}

impl StateManager {
    pub async fn init(
        pools: &HashSet<PoolId>,
        dex_info: Arc<DexInfo>,
    ) -> Result<(Self, Vec<(PoolId, VirtualReserves)>)> {
        let builder = StateManagerBuilder::from_pools(pools);

        let dex_info_clone1 = dex_info.clone();
        let dex_info_clone2 = dex_info.clone();
        let dex_info_clone3 = dex_info.clone();

        let (result_ethereum, result_bsc, result_arbitrum) = tokio::try_join!(
            tokio::spawn(async move {
                BlockchainStateManager::<evm_network::Ethereum>::init(
                    builder.protocol_addresses_ethereum,
                    dex_info_clone1,
                )
                .await
            }),
            tokio::spawn(async move {
                BlockchainStateManager::<evm_network::BSC>::init(
                    builder.protocol_addresses_bsc,
                    dex_info_clone2,
                )
                .await
            }),
            tokio::spawn(async move {
                BlockchainStateManager::<evm_network::Arbitrum>::init(
                    builder.protocol_addresses_arbitrum,
                    dex_info_clone3,
                )
                .await
            })
        )?;

        let (state_manager_ethereum, initial_reserves_ethereum) = result_ethereum?;
        let (state_manager_bsc, initial_reserves_bsc) = result_bsc?;
        let (state_manager_arbitrum, initial_reserves_arbitrum) = result_arbitrum?;

        Ok((
            Self {
                state_manager_ethereum,
                state_manager_bsc,
                state_manager_arbitrum,
            },
            vec![
                initial_reserves_ethereum,
                initial_reserves_bsc,
                initial_reserves_arbitrum,
            ]
            .into_iter()
            .flatten()
            .collect(),
        ))
    }

    pub async fn get_updated_reserves(self) -> Result<(Self, Vec<(PoolId, VirtualReserves)>)> {
        let (
            (state_manager_ethereum, reserves_ethereum),
            (state_manager_bsc, reserves_bsc),
            (state_manager_arbitrum, reserves_arbitrum),
        ) = tokio::try_join!(
            self.state_manager_ethereum.get_updated_reserves(),
            self.state_manager_bsc.get_updated_reserves(),
            self.state_manager_arbitrum.get_updated_reserves(),
        )?;

        Ok((
            Self {
                state_manager_ethereum,
                state_manager_bsc,
                state_manager_arbitrum,
            },
            vec![reserves_ethereum, reserves_bsc, reserves_arbitrum]
                .into_iter()
                .flatten()
                .collect(),
        ))
    }

    // pub fn new(dex_info: &'a DexInfo) -> Self {
    //     Self {
    //         state_manager_ethereum: BlockchainStateManager::new(),
    //         state_manager_bsc: BlockchainStateManager::new(),
    //         state_manager_arbitrum: BlockchainStateManager::new(),
    //     }
    // }

    // pub async fn subscribe_reserves(
    //     self,
    //     sender: mpsc::UnboundedSender<Vec<(PoolId, VirtualReserves)>>,
    //     tokens: Arc<HashMap<TokenAddress, TokenInfo>>,
    //     uniswap_v2_pools: Arc<HashMap<uniswap::v2::pool::PoolAddress, uniswap::v2::pool::PoolInfo>>,
    //     uniswap_v3_pools: Arc<HashMap<uniswap::v3::pool::PoolAddress, uniswap::v3::pool::PoolInfo>>,
    //     uniswap_v4_pools: Arc<HashMap<uniswap::v4::pool::PoolId, uniswap::v4::pool::PoolInfo>>,
    //     pancake_swap_pools: Arc<
    //         HashMap<pancakeswap::v3::pool::PoolAddress, pancakeswap::v3::pool::PoolInfo>,
    //     >,
    // ) -> Result<HashMap<PoolId, VirtualReserves>> {
    //     let (ethereum_join_handle, initial_reserves_ethereum) = subscribe_reserve_updates(
    //         sender.clone(),
    //         Arc::new(self.protocol_addresses_ethereum),
    //         tokens.clone(),
    //         uniswap_v2_pools.clone(),
    //         uniswap_v3_pools.clone(),
    //         uniswap_v4_pools.clone(),
    //         pancake_swap_pools.clone(),
    //     )
    //     .await?;
    //     let (bsc_join_handle, initial_reserves_bsc) = subscribe_reserve_updates(
    //         sender.clone(),
    //         Arc::new(self.protocol_addresses_bsc),
    //         tokens.clone(),
    //         uniswap_v2_pools.clone(),
    //         uniswap_v3_pools.clone(),
    //         uniswap_v4_pools.clone(),
    //         pancake_swap_pools.clone(),
    //     )
    //     .await?;
    //     let (arbitrum_join_handle, initial_reserves_arbitrum) = subscribe_reserve_updates(
    //         sender.clone(),
    //         Arc::new(self.protocol_addresses_arbitrum),
    //         tokens.clone(),
    //         uniswap_v2_pools.clone(),
    //         uniswap_v3_pools.clone(),
    //         uniswap_v4_pools.clone(),
    //         pancake_swap_pools.clone(),
    //     )
    //     .await?;

    //     // let combined_join_handle = tokio::spawn(async move {
    //     //     tokio::try_join!(ethereum_join_handle, bsc_join_handle, arbitrum_join_handle)
    //     //         .map_err(|_| anyhow!("Failed to join handles"))
    //     //         .and_then(|(a, b, c)| a.and(b).and(c))
    //     // });
    //     let combined_initial_reserves = initial_reserves_ethereum
    //         .into_iter()
    //         .chain(initial_reserves_bsc.into_iter())
    //         .chain(initial_reserves_arbitrum.into_iter())
    //         .collect::<HashMap<PoolId, VirtualReserves>>();
    //     Ok(combined_initial_reserves)
    // }

    // pub async fn subscribe_events(
    //     self,
    // ) -> Result<(Vec<(PoolId, VirtualReserves)>, JoinHandle<()>)> {
    //     let rpc_client_eth = rpc::client::init_client::<evm_network::Ethereum>().await?;
    //     let rpc_client_bsc = rpc::client::init_client::<evm_network::BSC>().await?;
    //     let rpc_client_arb = rpc::client::init_client::<evm_network::Arbitrum>().await?;

    //     let reserve_calls_eth = self
    //         .state_manager_ethereum
    //         .protocol_addresses
    //         .initial_calls(
    //             &self.tokens,
    //             &self.uniswap_v2_pools,
    //             &self.uniswap_v3_pools,
    //             &self.uniswap_v4_pools,
    //             &self.pancake_swap_pools,
    //         )?;
    //     let reserve_calls_bsc = self.state_manager_bsc.protocol_addresses.initial_calls(
    //         &self.tokens,
    //         &self.uniswap_v2_pools,
    //         &self.uniswap_v3_pools,
    //         &self.uniswap_v4_pools,
    //         &self.pancake_swap_pools,
    //     )?;
    //     let reserve_calls_arb = self
    //         .state_manager_arbitrum
    //         .protocol_addresses
    //         .initial_calls(
    //             &self.tokens,
    //             &self.uniswap_v2_pools,
    //             &self.uniswap_v3_pools,
    //             &self.uniswap_v4_pools,
    //             &self.pancake_swap_pools,
    //         )?;

    //     let mut receiver_eth = self.state_manager_ethereum.subscribe_events()?;
    //     let mut receiver_bsc = self.state_manager_bsc.subscribe_events()?;
    //     let mut receiver_arb = self.state_manager_arbitrum.subscribe_events()?;

    //     let (first_events_batch_eth, first_events_batch_bsc, first_events_batch_arb) = tokio::join!(
    //         receiver_eth.recv(),
    //         receiver_bsc.recv(),
    //         receiver_arb.recv()
    //     );

    //     let block_number_eth = first_events_batch_eth
    //         .ok_or_else(|| anyhow::anyhow!("Failed to receive first events batch from Ethereum"))
    //         .and_then(|events| {
    //             events
    //                 .into_iter()
    //                 .map(|event| event.info.block_number)
    //                 .reduce(BlockNumber::pick_latest)
    //                 .ok_or_else(|| anyhow::anyhow!("Failed to pick latest block number from Ethereum events: Empty events batch"))
    //         })?;
    //     let block_number_bsc = first_events_batch_bsc
    //         .ok_or_else(|| anyhow::anyhow!("Failed to receive first events batch from BSC"))
    //         .and_then(|events| {
    //             events
    //                 .into_iter()
    //                 .map(|event| event.info.block_number)
    //                 .reduce(BlockNumber::pick_latest)
    //                 .ok_or_else(|| {
    //                     anyhow::anyhow!(
    //                         "Failed to pick latest block number from BSC events: Empty events batch"
    //                     )
    //                 })
    //         })?;
    //     let block_number_arb = first_events_batch_arb
    //         .ok_or_else(|| anyhow::anyhow!("Failed to receive first events batch from Arbitrum"))
    //         .and_then(|events| {
    //             events
    //                 .into_iter()
    //                 .map(|event| event.info.block_number)
    //                 .reduce(BlockNumber::pick_latest)
    //                 .ok_or_else(|| anyhow::anyhow!("Failed to pick latest block number from Arbitrum events: Empty events batch"))
    //         })?;

    //     let eth_call = async move {
    //         let block_number = rpc_client_eth.get_block_number().await?;
    //         let reserves_eth = rpc_client_eth
    //             .get_multicall_chunked(&reserve_calls_eth, block_number, 1000)
    //             .await?;

    //         let reserves_eth = reserves_eth.into_iter().collect::<Result<Vec<_>, _>>()?;
    //         anyhow::Ok(reserves_eth)
    //     };

    //     let bsc_call = async move {
    //         let block_number = rpc_client_bsc.get_block_number().await?;
    //         let reserves_bsc = rpc_client_bsc
    //             .get_multicall_chunked(&reserve_calls_bsc, block_number, 1000)
    //             .await?;

    //         let reserves_bsc = reserves_bsc.into_iter().collect::<Result<Vec<_>, _>>()?;
    //         anyhow::Ok(reserves_bsc)
    //     };

    //     let arb_call = async move {
    //         let block_number = rpc_client_arb.get_block_number().await?;
    //         let reserves_arb = rpc_client_arb
    //             .get_multicall_chunked(&reserve_calls_arb, block_number, 1000)
    //             .await?;

    //         let reserves_arb = reserves_arb.into_iter().collect::<Result<Vec<_>, _>>()?;
    //         anyhow::Ok(reserves_arb)
    //     };

    //     let handle: JoinHandle<()> = tokio::spawn(async move {
    //         loop {
    //             let size_eth = receiver_eth.len();
    //             let mut buffer_eth = Vec::with_capacity(size_eth);
    //             let eth = receiver_eth.recv_many(&mut buffer_eth, size_eth);

    //             let size_bsc = receiver_bsc.len();
    //             let mut buffer_bsc = Vec::with_capacity(size_bsc);
    //             let bsc = receiver_bsc.recv_many(&mut buffer_bsc, size_bsc);

    //             let size_arb = receiver_arb.len();
    //             let mut buffer_arb = Vec::with_capacity(size_arb);
    //             let arb = receiver_arb.recv_many(&mut buffer_arb, size_arb);

    //             let (rc_eth, rc_bsc, rc_arb) = tokio::join!(eth, bsc, arb);

    //             println!(
    //                 "Ethereum logs: {}, BSC logs: {}, Arbitrum logs: {}",
    //                 buffer_eth.len(),
    //                 buffer_bsc.len(),
    //                 buffer_arb.len()
    //             );

    //             tokio::time::sleep(Duration::from_secs(3)).await;
    //         }
    //     });

    //     println!("Requesting initial reserves");
    //     let (reserves_eth, reserves_bsc, reserves_arb) =
    //         tokio::try_join!(eth_call, bsc_call, arb_call)?;
    //     // let handle = tokio::spawn(async move {
    //     //     // let (rpc_client_ethereum, rpc_client_bsc, rpc_client_arbitrum) = tokio::try_join!(
    //     //     //     rpc::client::init_client::<evm_network::Ethereum>(),
    //     //     //     rpc::client::init_client::<evm_network::BSC>(),
    //     //     //     rpc::client::init_client::<evm_network::Arbitrum>()
    //     //     // )?;
    //     //     println!("Starting event aggregation loop");
    //     //     loop {
    //     //         tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    //     //         println!("Requesting aggregated events");

    //     //         let requests_sender_ethereum_clone = requests_sender_ethereum.clone();
    //     //         let requests_sender_bsc_clone = requests_sender_bsc.clone();
    //     //         let requests_sender_arbitrum_clone = requests_sender_arbitrum.clone();

    //     //         let (events_ethereum, events_bsc, events_arbitrum) = tokio::try_join!(
    //     //             async move {
    //     //                 let (response_sender_ethereum, response_receiver_ethereum) =
    //     //                     oneshot::channel::<Vec<Event<evm_network::Ethereum>>>();
    //     //                 requests_sender_ethereum_clone
    //     //                     .send(response_sender_ethereum)
    //     //                     .await?;
    //     //                 let response = response_receiver_ethereum.await?;
    //     //                 anyhow::Ok(response)
    //     //             },
    //     //             async move {
    //     //                 let (response_sender_bsc, response_receiver_bsc) =
    //     //                     oneshot::channel::<Vec<Event<evm_network::BSC>>>();
    //     //                 requests_sender_bsc_clone.send(response_sender_bsc).await?;
    //     //                 let response = response_receiver_bsc.await?;
    //     //                 anyhow::Ok(response)
    //     //             },
    //     //             async move {
    //     //                 let (response_sender_arbitrum, response_receiver_arbitrum) =
    //     //                     oneshot::channel::<Vec<Event<evm_network::Arbitrum>>>();
    //     //                 requests_sender_arbitrum_clone
    //     //                     .send(response_sender_arbitrum)
    //     //                     .await?;
    //     //                 let response = response_receiver_arbitrum.await?;
    //     //                 anyhow::Ok(response)
    //     //             }
    //     //         )?;

    //     //         println!(
    //     //             "Ethereum events: {}, BSC events: {}, Arbitrum events: {}",
    //     //             events_ethereum.len(),
    //     //             events_bsc.len(),
    //     //             events_arbitrum.len()
    //     //         );
    //     //     }

    //     //     anyhow::Ok(())
    //     // });
    //     anyhow::Ok((
    //         vec![reserves_eth, reserves_bsc, reserves_arb]
    //             .into_iter()
    //             .flatten()
    //             .collect(),
    //         handle,
    //     ))
    // }
}

// async fn subscribe_blockchain_topics<B: BlockchainNetwork + RecommendedFillers>(
//     protocol_addresses: Arc<ProtocolAddresses<B>>,
// ) -> Result<UnboundedReceiver<EventId>> {
//     let (events_sender, events_receiver) = mpsc::unbounded_channel::<EventId>();

//     rpc::client::subscribe_topics::<EventId, B>(
//         events_sender.clone(),
//         HashSet::from(TOPICS),
//         move |log| protocol_addresses.try_lookup(log).unwrap().map(|e| e.id), // TODO: Handle event parsing errors
//     )
//     .await?;

//     Ok(events_receiver)
// }

// async fn subscribe_reserve_updates<B: BlockchainNetwork + RecommendedFillers>(
//     sender: mpsc::UnboundedSender<Vec<(PoolId, VirtualReserves)>>,
//     protocol_addresses: Arc<ProtocolAddresses<B>>,
//     tokens: Arc<HashMap<TokenAddress, TokenInfo>>,
//     uniswap_v2_pools: Arc<HashMap<uniswap::v2::pool::PoolAddress, uniswap::v2::pool::PoolInfo>>,
//     uniswap_v3_pools: Arc<HashMap<uniswap::v3::pool::PoolAddress, uniswap::v3::pool::PoolInfo>>,
//     uniswap_v4_pools: Arc<HashMap<uniswap::v4::pool::PoolId, uniswap::v4::pool::PoolInfo>>,
//     pancake_swap_pools: Arc<
//         HashMap<pancakeswap::v3::pool::PoolAddress, pancakeswap::v3::pool::PoolInfo>,
//     >,
// ) -> Result<(JoinHandle<Result<()>>, Vec<(PoolId, VirtualReserves)>)> {
//     println!("Subscribing to pool updates on {}", {
//         B::BLOCKCHAIN.name()
//     });
//     let (events_sender, mut events_receiver) = mpsc::unbounded_channel::<Event<B>>();

//     let protocol_addresses_clone = protocol_addresses.clone();

//     let rpc_client = rpc::client::init_client::<B>().await?;

//     rpc::client::subscribe_topics::<Event<B>, B>(
//         events_sender.clone(),
//         HashSet::from(TOPICS),
//         move |log| protocol_addresses_clone.try_lookup(log).unwrap(), // TODO: Handle event parsing errors
//     )
//     .await?;

//     let block_number = rpc_client.get_block_number().await?;
//     let initial_calls = protocol_addresses.initial_calls(
//         tokens.as_ref(),
//         uniswap_v2_pools.as_ref(),
//         uniswap_v3_pools.as_ref(),
//         uniswap_v4_pools.as_ref(),
//         pancake_swap_pools.as_ref(),
//     )?;

//     println!("Loading initial reserves");
//     let initial_reserves = futures_util::stream::iter(initial_calls.chunks(1000))
//         .map(Ok::<&[ReservesCallData<B>], anyhow::Error>)
//         .try_fold(
//             Vec::<(PoolId, VirtualReserves)>::new(),
//             async |mut combined_reserves, chunk| {
//                 let reserves = rpc_client
//                     .get_multicall(chunk, block_number)
//                     .await?
//                     .into_iter()
//                     .collect::<Result<Vec<_>, _>>()?;
//                 combined_reserves.extend(reserves);
//                 Ok(combined_reserves)
//             },
//         )
//         .await?;

//     println!(
//         "Loaded {} initial reserves on {}",
//         initial_reserves.len(),
//         B::BLOCKCHAIN
//     );

//     let handle = tokio::spawn(async move {
//         loop {
//             let mut buffer = Vec::new();
//             let received_count = events_receiver.recv_many(&mut buffer, 1000).await;

//             println!(
//                 "{} events received on {}",
//                 received_count,
//                 B::BLOCKCHAIN.name()
//             );
//             if received_count == 0 {
//                 println!("Logs subscription terminated");
//                 break;
//             }
//             // if let Some((head, tail)) = buffer.split_first() {
//             //     let (events_map, block_number) = tail.into_iter().fold(
//             //         (
//             //             HashMap::<EventId, &EventInfo<B>>::from([(head.id, &head.info)]),
//             //             head.info.block_number,
//             //         ),
//             //         |(mut events, latest_block_number), event| {
//             //             events
//             //                 .entry(event.id)
//             //                 .and_modify(|existing| {
//             //                     if existing.block_number.value() < event.info.block_number.value() {
//             //                         *existing = &event.info;
//             //                     }
//             //                 })
//             //                 .or_insert_with(|| &event.info);
//             //             (
//             //                 events,
//             //                 BlockNumber::pick_latest(latest_block_number, event.info.block_number),
//             //             )
//             //         },
//             //     );

//             //     let calls = events_map
//             //         .into_iter()
//             //         .map(|(event_id, _)| {
//             //             event_id.into_call_data(
//             //                 tokens.as_ref(),
//             //                 uniswap_v2_pools.as_ref(),
//             //                 uniswap_v3_pools.as_ref(),
//             //                 uniswap_v4_pools.as_ref(),
//             //                 pancake_swap_pools.as_ref(),
//             //             )
//             //         })
//             //         .collect::<Result<Vec<_>>>()?;

//             //     let updated_reserves = rpc_client
//             //         .get_multicall(&calls, block_number)
//             //         .await?
//             //         .into_iter()
//             //         .collect::<Result<Vec<_>, _>>()?;

//             //     sender.send(updated_reserves)?;
//             // } else {
//             //     println!("Logs subscription terminated");
//             //     break;
//             // }
//         }
//         Ok(())
//     });

//     Ok((handle, initial_reserves))
// }
