use std::{collections::HashMap, marker::PhantomData};

use alloy::{
    primitives::{Address, FixedBytes},
    providers::fillers::RecommendedFillers,
    rpc::types::Log,
    sol_types::SolEvent,
};
use anyhow::{Result, anyhow};

use crate::{
    DexInfo,
    blockchain::{BlockNumber, BlockchainNetwork},
    state_manager::{
        event::{Event, EventId},
        pool_reserves_calls::ReservesCallData,
    },
    uniswap_internal as uniswap,
};

fn try_get_topic1(log: Log) -> Result<FixedBytes<32>> {
    log.topics()
        .get(1)
        .map(|topic| *topic)
        .ok_or(anyhow!("Failed to read topic1 from the Log\n{:?}", log))
}

pub struct ProtocolAddresses<B: BlockchainNetwork + RecommendedFillers> {
    v2: HashMap<Address, fn(Address) -> EventId>,
    v3: HashMap<Address, fn(Address) -> EventId>,
    v4: HashMap<FixedBytes<32>, fn(FixedBytes<32>) -> EventId>,
    blockchain_marker: PhantomData<B>,
}

impl<B: BlockchainNetwork + RecommendedFillers> ProtocolAddresses<B> {
    pub fn new() -> Self {
        Self {
            v2: HashMap::new(),
            v3: HashMap::new(),
            v4: HashMap::new(),
            blockchain_marker: PhantomData,
        }
    }

    pub fn with_v2_address(mut self, address: Address, ctor: fn(Address) -> EventId) -> Self {
        self.v2.insert(address, ctor);
        self
    }

    pub fn with_v3_address(mut self, address: Address, ctor: fn(Address) -> EventId) -> Self {
        self.v3.insert(address, ctor);
        self
    }

    pub fn with_v4_id(mut self, id: FixedBytes<32>, ctor: fn(FixedBytes<32>) -> EventId) -> Self {
        self.v4.insert(id, ctor);
        self
    }

    pub fn initial_calls(&self, dex_info: &DexInfo) -> Result<Vec<ReservesCallData<B>>> {
        self.v2
            .iter()
            .map(|(address, ctor)| ctor(*address))
            .chain(self.v3.iter().map(|(address, ctor)| ctor(*address)))
            .chain(self.v4.iter().map(|(id, ctor)| ctor(*id)))
            .map(|id: EventId| id.into_call_data(dex_info))
            .collect::<Result<Vec<_>>>()
    }

    pub fn try_lookup(&self, log: Log) -> Result<Option<Event<B>>> {
        let block_number =
            BlockNumber::<B>::new(log.block_number.ok_or(anyhow!("Missing block number"))?);
        match log.topic0() {
            // V2 protocol
            Some(&uniswap::v2::contract::Pair::Swap::SIGNATURE_HASH) => Ok(self
                .v2
                .get(&log.address())
                .map(|construct_id_fn| Event::new(construct_id_fn(log.address()), block_number))),
            Some(&uniswap::v2::contract::Pair::Mint::SIGNATURE_HASH) => Ok(self
                .v2
                .get(&log.address())
                .map(|construct_id_fn| Event::new(construct_id_fn(log.address()), block_number))),
            Some(&uniswap::v2::contract::Pair::Burn::SIGNATURE_HASH) => Ok(self
                .v2
                .get(&log.address())
                .map(|construct_id_fn| Event::new(construct_id_fn(log.address()), block_number))),
            Some(&uniswap::v2::contract::Pair::Sync::SIGNATURE_HASH) => Ok(self
                .v2
                .get(&log.address())
                .map(|construct_id_fn| Event::new(construct_id_fn(log.address()), block_number))),

            // V3 protocol
            Some(&uniswap::v3::contract::Pool::Swap::SIGNATURE_HASH) => Ok(self
                .v3
                .get(&log.address())
                .map(|construct_id_fn| Event::new(construct_id_fn(log.address()), block_number))),
            Some(&uniswap::v3::contract::Pool::Mint::SIGNATURE_HASH) => Ok(self
                .v3
                .get(&log.address())
                .map(|construct_id_fn| Event::new(construct_id_fn(log.address()), block_number))),
            Some(&uniswap::v3::contract::Pool::Burn::SIGNATURE_HASH) => Ok(self
                .v3
                .get(&log.address())
                .map(|construct_id_fn| Event::new(construct_id_fn(log.address()), block_number))),

            // V4 protocol
            Some(&uniswap::v4::contract::PoolManager::ModifyLiquidity::SIGNATURE_HASH) => {
                let id = try_get_topic1(log)?;
                Ok(self
                    .v4
                    .get(&id)
                    .map(|construct_id_fn| Event::new(construct_id_fn(id), block_number)))
            }
            Some(&uniswap::v4::contract::PoolManager::Swap::SIGNATURE_HASH) => {
                let id = try_get_topic1(log)?;
                Ok(self
                    .v4
                    .get(&id)
                    .map(|construct_id_fn| Event::new(construct_id_fn(id), block_number)))
            }
            Some(&uniswap::v4::contract::PoolManager::Donate::SIGNATURE_HASH) => {
                let id = try_get_topic1(log)?;
                Ok(self
                    .v4
                    .get(&id)
                    .map(|construct_id_fn| Event::new(construct_id_fn(id), block_number)))
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
