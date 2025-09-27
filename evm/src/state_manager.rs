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
    tokens::Token,
    uniswap_internal as uniswap,
    virtual_reserves::VirtualReserves,
};

enum ReservesCallData<B: BlockchainNetwork> {
    UniswapV2(uniswap::v2::reserves_call_data::ReservesCallData<B>),
    UniswapV3(uniswap::v3::reserves_call_data::ReservesCallData<B>),
    UniswapV4(uniswap::v4::reserves_call_data::ReservesCallData<B>),
    PancakeSwap(pancakeswap::v3::reserves_call_data::ReservesCallData<B>),
}

impl<B: BlockchainNetwork> MulticallData<B> for ReservesCallData<B> {
    type Calls = Vec<multicall::Multicall3::Call>;
    type Output = VirtualReserves;

    fn to_calls(&self) -> Self::Calls {
        match self {
            ReservesCallData::UniswapV2(data) => data.to_calls().into_iter().collect(),
            ReservesCallData::UniswapV3(data) => data.to_calls().into_iter().collect(),
            ReservesCallData::UniswapV4(data) => data.to_calls().into_iter().collect(),
            ReservesCallData::PancakeSwap(data) => data.to_calls().into_iter().collect(),
        }
    }

    fn decode_output(&self, response: &[Bytes]) -> Result<Self::Output> {
        match self {
            ReservesCallData::UniswapV2(data) => data.decode_output(response),
            ReservesCallData::UniswapV3(data) => data.decode_output(response),
            ReservesCallData::UniswapV4(data) => data.decode_output(response),
            ReservesCallData::PancakeSwap(data) => data.decode_output(response),
        }
    }

    fn size(&self) -> usize {
        match self {
            ReservesCallData::UniswapV2(data) => data.size(),
            ReservesCallData::UniswapV3(data) => data.size(),
            ReservesCallData::UniswapV4(data) => data.size(),
            ReservesCallData::PancakeSwap(data) => data.size(),
        }
    }
}

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
        ids: &[PoolId],
        lookup_tokens: impl Fn(PoolId) -> (Token, Token),
    ) -> Result<HashMap<PoolId, VirtualReserves>> {
        Ok(HashMap::new())
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
