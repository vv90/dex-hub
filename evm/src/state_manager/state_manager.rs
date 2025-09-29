use alloy::{
    primitives::{Address, Bytes, FixedBytes},
    providers::fillers::RecommendedFillers,
    rpc::types::Log,
    sol_types::SolEvent,
};
use anyhow::{Result, anyhow};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    Blockchain,
    blockchain::BlockchainNetwork,
    evm_network, multicall, pancakeswap_internal as pancakeswap,
    pool_id::PoolId,
    rpc::{self, client::NetworkProvider, multicall_data::MulticallData},
    state_manager::pool_reserves_calls::PoolReservesCalls,
    tokens::Token,
    uniswap_internal as uniswap,
    virtual_reserves::VirtualReserves,
};

type RpcClientEthereum =
    rpc::client::RpcClient<evm_network::Ethereum, NetworkProvider<evm_network::Ethereum>>;
type RpcClientBSC = rpc::client::RpcClient<evm_network::BSC, NetworkProvider<evm_network::BSC>>;
type RpcClientArbitrum =
    rpc::client::RpcClient<evm_network::Arbitrum, NetworkProvider<evm_network::Arbitrum>>;

fn uniswap_v2_id(address: Address, blockchain: Blockchain) -> PoolId {
    PoolId::UniswapV2(uniswap::v2::pool::PoolAddress(address, blockchain))
}

fn uniswap_v3_id(address: Address, blockchain: Blockchain) -> PoolId {
    PoolId::UniswapV3(uniswap::v3::pool::PoolAddress(address, blockchain))
}

fn uniswap_v4_id(id: FixedBytes<32>, blockchain: Blockchain) -> PoolId {
    PoolId::UniswapV4(uniswap::v4::pool::PoolId(id, blockchain))
}

fn pancakeswap_v3_id(address: Address, blockchain: Blockchain) -> PoolId {
    PoolId::PancakeSwap(pancakeswap::v3::pool::PoolAddress(address, blockchain))
}

fn try_get_topic1(log: Log) -> Result<FixedBytes<32>> {
    log.topics()
        .get(1)
        .map(|topic| *topic)
        .ok_or(anyhow!("Failed to read topic1 from the Log\n{:?}", log))
}

pub struct StateManager {
    states: HashMap<PoolId, Option<VirtualReserves>>,
    rpc_client_ethereum: RpcClientEthereum,
    rpc_client_bsc: RpcClientBSC,
    rpc_client_arbitrum: RpcClientArbitrum,
}

struct ProtocolAddresses {
    blockchain: Blockchain,
    v2: HashMap<Address, fn(Address, Blockchain) -> PoolId>,
    v3: HashMap<Address, fn(Address, Blockchain) -> PoolId>,
    v4: HashMap<FixedBytes<32>, fn(FixedBytes<32>, Blockchain) -> PoolId>,
}

impl ProtocolAddresses {
    pub fn new(blockchain: Blockchain) -> Self {
        Self {
            blockchain,
            v2: HashMap::new(),
            v3: HashMap::new(),
            v4: HashMap::new(),
        }
    }

    pub fn lookup(&self, log: Log) -> Result<Option<PoolId>> {
        match log.topic0() {
            // V2 protocol
            Some(&uniswap::v2::contract::Pair::Swap::SIGNATURE_HASH) => Ok(self
                .v2
                .get(&log.address())
                .map(|construct_id_fn| construct_id_fn(log.address(), self.blockchain))),
            Some(&uniswap::v2::contract::Pair::Mint::SIGNATURE_HASH) => Ok(self
                .v2
                .get(&log.address())
                .map(|construct_id_fn| construct_id_fn(log.address(), self.blockchain))),
            Some(&uniswap::v2::contract::Pair::Burn::SIGNATURE_HASH) => Ok(self
                .v2
                .get(&log.address())
                .map(|construct_id_fn| construct_id_fn(log.address(), self.blockchain))),
            Some(&uniswap::v2::contract::Pair::Sync::SIGNATURE_HASH) => Ok(self
                .v2
                .get(&log.address())
                .map(|construct_id_fn| construct_id_fn(log.address(), self.blockchain))),

            // V3 protocol
            Some(&uniswap::v3::contract::Pool::Swap::SIGNATURE_HASH) => Ok(self
                .v3
                .get(&log.address())
                .map(|construct_id_fn| construct_id_fn(log.address(), self.blockchain))),
            Some(&uniswap::v3::contract::Pool::Mint::SIGNATURE_HASH) => Ok(self
                .v3
                .get(&log.address())
                .map(|construct_id_fn| construct_id_fn(log.address(), self.blockchain))),
            Some(&uniswap::v3::contract::Pool::Burn::SIGNATURE_HASH) => Ok(self
                .v3
                .get(&log.address())
                .map(|construct_id_fn| construct_id_fn(log.address(), self.blockchain))),

            // V4 protocol
            Some(&uniswap::v4::contract::PoolManager::ModifyLiquidity::SIGNATURE_HASH) => {
                let id = try_get_topic1(log)?;
                Ok(self
                    .v4
                    .get(&id)
                    .map(|construct_id_fn| construct_id_fn(id, self.blockchain)))
            }
            Some(&uniswap::v4::contract::PoolManager::Swap::SIGNATURE_HASH) => {
                let id = try_get_topic1(log)?;
                Ok(self
                    .v4
                    .get(&id)
                    .map(|construct_id_fn| construct_id_fn(id, self.blockchain)))
            }
            Some(&uniswap::v4::contract::PoolManager::Donate::SIGNATURE_HASH) => {
                let id = try_get_topic1(log)?;
                Ok(self
                    .v4
                    .get(&id)
                    .map(|construct_id_fn| construct_id_fn(id, self.blockchain)))
            }

            Some(unknown_event_hash) => Err(anyhow!(
                "Unknown event hash {} in the log\n{:?}",
                unknown_event_hash,
                log
            )),
            None => Err(anyhow!("Failed to read topic0 from the Log\n{:?}", log)),
        }
    }
}

struct PoolAddressesMap {
    ethereum: ProtocolAddresses,
    bsc: ProtocolAddresses,
    arbitrum: ProtocolAddresses,
}

impl PoolAddressesMap {
    fn new() -> Self {
        Self {
            ethereum: ProtocolAddresses::new(Blockchain::Ethereum),
            bsc: ProtocolAddresses::new(Blockchain::BSC),
            arbitrum: ProtocolAddresses::new(Blockchain::Arbitrum),
        }
    }

    fn get(&mut self, blockchain: Blockchain) -> &mut ProtocolAddresses {
        match blockchain {
            Blockchain::Ethereum => &mut self.ethereum,
            Blockchain::BSC => &mut self.bsc,
            Blockchain::Arbitrum => &mut self.arbitrum,
        }
    }
}

impl StateManager {
    async fn get_reserves(
        &self,
        ids: &[PoolId],
        lookup_uniswap_v2_pool_info: impl Fn(
            &uniswap::v2::pool::PoolAddress,
        ) -> Option<&uniswap::v2::pool::PoolInfo>,
        lookup_uniswap_v3_pool_info: impl Fn(
            &uniswap::v3::pool::PoolAddress,
        ) -> Option<&uniswap::v3::pool::PoolInfo>,
        lookup_uniswap_v4_pool_info: impl Fn(
            &uniswap::v4::pool::PoolId,
        ) -> Option<&uniswap::v4::pool::PoolInfo>,
        lookup_pancakeswap_pool_info: impl Fn(
            &pancakeswap::v3::pool::PoolAddress,
        ) -> Option<&pancakeswap::v3::pool::PoolInfo>,
    ) -> Result<HashMap<PoolId, VirtualReserves>> {
        let PoolReservesCalls {
            ethereum,
            bsc,
            arbitrum,
        } = ids.into_iter().try_fold(
            PoolReservesCalls::new(),
            |calls, id| -> Result<PoolReservesCalls> {
                match id {
                    PoolId::UniswapV2(pool_address) => calls.with_uniswap_v2_call(
                        pool_address,
                        lookup_uniswap_v2_pool_info(pool_address)
                            .ok_or(anyhow!("pool info not found for pool id {:?}", id))?,
                    ),
                    PoolId::UniswapV3(pool_address) => calls.with_uniswap_v3_call(
                        pool_address,
                        lookup_uniswap_v3_pool_info(pool_address)
                            .ok_or(anyhow!("pool info not found for pool id {:?}", id))?,
                    ),
                    PoolId::UniswapV4(pool_id) => calls.with_uniswap_v4_call(
                        pool_id,
                        lookup_uniswap_v4_pool_info(pool_id)
                            .ok_or(anyhow!("pool info not found for pool id {:?}", id))?,
                    ),
                    PoolId::PancakeSwap(pool_address) => calls.with_pancakeswap_v3_call(
                        pool_address,
                        lookup_pancakeswap_pool_info(pool_address)
                            .ok_or(anyhow!("pool info not found for pool id {:?}", id))?,
                    ),
                }
            },
        )?;

        let ethereum_block_number = self.rpc_client_ethereum.get_block_number().await?;
        let ethereum_reserves = self
            .rpc_client_ethereum
            .get_multicall(&ethereum, ethereum_block_number)
            .await?
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        let bsc_block_number = self.rpc_client_bsc.get_block_number().await?;
        let bsc_reserves = self
            .rpc_client_bsc
            .get_multicall(&bsc, bsc_block_number)
            .await?
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        let arbitrum_block_number = self.rpc_client_arbitrum.get_block_number().await?;
        let arbitrum_reserves = self
            .rpc_client_arbitrum
            .get_multicall(&arbitrum, arbitrum_block_number)
            .await?
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        Ok(ethereum_reserves
            .into_iter()
            .chain(bsc_reserves.into_iter())
            .chain(arbitrum_reserves.into_iter())
            .collect())
    }

    pub async fn init(ids: &[PoolId]) -> Result<Self> {
        let (sender, receiver) = mpsc::unbounded_channel::<PoolId>(); // TODO: use bounded channel and handle channel saturation

        let states = ids.iter().map(|id| (*id, None)).collect();

        let rpc_client_ethereum: RpcClientEthereum = rpc::client::init_client().await?;
        let rpc_client_bsc: RpcClientBSC = rpc::client::init_client().await?;
        let rpc_client_arbitrum: RpcClientArbitrum = rpc::client::init_client().await?;

        let PoolAddressesMap {
            ethereum,
            bsc,
            arbitrum,
        } = ids
            .iter()
            .fold(PoolAddressesMap::new(), |mut blockchain_maps, &id| {
                match id {
                    PoolId::UniswapV2(uniswap::v2::pool::PoolAddress(address, blockchain)) => {
                        blockchain_maps
                            .get(blockchain)
                            .v2
                            .entry(address)
                            .or_insert(uniswap_v2_id);
                    }
                    PoolId::UniswapV3(uniswap::v3::pool::PoolAddress(address, blockchain)) => {
                        blockchain_maps
                            .get(blockchain)
                            .v3
                            .entry(address)
                            .or_insert(uniswap_v3_id);
                    }
                    PoolId::PancakeSwap(pancakeswap::v3::pool::PoolAddress(
                        address,
                        blockchain,
                    )) => {
                        blockchain_maps
                            .get(blockchain)
                            .v3
                            .entry(address)
                            .or_insert(pancakeswap_v3_id);
                    }
                    PoolId::UniswapV4(uniswap::v4::pool::PoolId(address, blockchain)) => {
                        blockchain_maps
                            .get(blockchain)
                            .v4
                            .entry(address)
                            .or_insert(uniswap_v4_id);
                    }
                };
                blockchain_maps
            });

        let topics = HashSet::from([
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
        ]);

        let ethereum_map_arc = Arc::new(ethereum);
        let bsc_map_arc = Arc::new(bsc);
        let arbitrum_map_arc = Arc::new(arbitrum);

        let sub_eth_handle = rpc::client::subscribe_topics::<PoolId, evm_network::Ethereum>(
            sender.clone(),
            topics.clone(),
            move |log| ethereum_map_arc.clone().lookup(log).unwrap(), // TODO: Handle event parsing errors
        )
        .await?;

        let sub_bsc_handle = rpc::client::subscribe_topics::<PoolId, evm_network::BSC>(
            sender.clone(),
            topics.clone(),
            move |log| bsc_map_arc.clone().lookup(log).unwrap(), // TODO: Handle event parsing errors
        )
        .await?;

        let sub_arbitrum_handle = rpc::client::subscribe_topics::<PoolId, evm_network::Arbitrum>(
            sender.clone(),
            topics.clone(),
            move |log| arbitrum_map_arc.clone().lookup(log).unwrap(), // TODO: Handle event parsing errors
        )
        .await?;

        Ok(Self {
            states,
            rpc_client_ethereum,
            rpc_client_bsc,
            rpc_client_arbitrum,
        })
    }
}
