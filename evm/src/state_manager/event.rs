use crate::{
    DexInfo,
    blockchain::{BlockNumber, BlockchainNetwork},
    pancakeswap_internal as pancakeswap,
    state_manager::pool_reserves_calls::ReservesCallData,
    uniswap_internal as uniswap,
};
use alloy::primitives::{Address, FixedBytes};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventId {
    UniswapV2(Address),
    UniswapV3(Address),
    UniswapV4(FixedBytes<32>),
    PancakeSwap(Address),
}

pub struct EventInfo<B: BlockchainNetwork> {
    pub block_number: BlockNumber<B>,
}

impl<B: BlockchainNetwork> EventInfo<B> {
    pub fn new(block_number: BlockNumber<B>) -> Self {
        Self { block_number }
    }
}

pub struct Event<B: BlockchainNetwork> {
    pub id: EventId,
    pub info: EventInfo<B>,
}

impl<B: BlockchainNetwork> Event<B> {
    pub fn new(id: EventId, block_number: BlockNumber<B>) -> Self {
        Self {
            id,
            info: EventInfo::<B>::new(block_number),
        }
    }
}

impl EventId {
    pub fn into_call_data<B: BlockchainNetwork>(
        self,
        dex_info: &DexInfo,
    ) -> Result<ReservesCallData<B>> {
        match self {
            EventId::UniswapV2(address) => {
                let pool_info = dex_info
                    .uniswap_v2_pools
                    .get(&uniswap::v2::pool::PoolAddress(address, B::BLOCKCHAIN))
                    .ok_or(anyhow!("UniswapV2 pool {:?} info not found", address))?;

                let token0 = dex_info
                    .tokens
                    .get(&pool_info.token0)
                    .ok_or(anyhow!("Token {:?} not found", pool_info.token0))?;
                let token1 = dex_info
                    .tokens
                    .get(&pool_info.token1)
                    .ok_or(anyhow!("Token {:?} not found", pool_info.token1))?;

                Ok(ReservesCallData::UniswapV2(
                    uniswap::v2::reserves_call_data::ReservesCallData::new(address, token0, token1),
                ))
            }
            EventId::UniswapV3(address) => {
                let pool_info = dex_info
                    .uniswap_v3_pools
                    .get(&uniswap::v3::pool::PoolAddress(address, B::BLOCKCHAIN))
                    .ok_or(anyhow!("UniswapV3 pool {:?} info not found", address))?;

                let token0 = dex_info
                    .tokens
                    .get(&pool_info.token0)
                    .ok_or(anyhow!("Token {:?} not found", pool_info.token0))?;
                let token1 = dex_info
                    .tokens
                    .get(&pool_info.token1)
                    .ok_or(anyhow!("Token {:?} not found", pool_info.token1))?;

                Ok(ReservesCallData::UniswapV3(
                    uniswap::v3::reserves_call_data::ReservesCallData::new(
                        address,
                        token0,
                        token1,
                        pool_info.fee,
                    ),
                ))
            }
            EventId::UniswapV4(id) => {
                let pool_info = dex_info
                    .uniswap_v4_pools
                    .get(&uniswap::v4::pool::PoolId(id, B::BLOCKCHAIN))
                    .ok_or(anyhow!("UniswapV4 pool {:?} info not found", id))?;

                let token0 = dex_info
                    .tokens
                    .get(&pool_info.token0)
                    .ok_or(anyhow!("Token {:?} not found", pool_info.token0))?;
                let token1 = dex_info
                    .tokens
                    .get(&pool_info.token1)
                    .ok_or(anyhow!("Token {:?} not found", pool_info.token1))?;

                Ok(ReservesCallData::UniswapV4(
                    uniswap::v4::reserves_call_data::ReservesCallData::new(
                        id,
                        token0,
                        token1,
                        pool_info.fee,
                        pool_info.tick_spacing,
                    ),
                ))
            }
            EventId::PancakeSwap(address) => {
                let pool_info = dex_info
                    .pancakeswap_pools
                    .get(&pancakeswap::v3::pool::PoolAddress(address, B::BLOCKCHAIN))
                    .ok_or(anyhow!("PancakeSwap pool {:?} info not found", address))?;

                let token0 = dex_info
                    .tokens
                    .get(&pool_info.token0)
                    .ok_or(anyhow!("Token {:?} not found", pool_info.token0))?;

                let token1 = dex_info
                    .tokens
                    .get(&pool_info.token1)
                    .ok_or(anyhow!("Token {:?} not found", pool_info.token1))?;

                Ok(ReservesCallData::PancakeSwap(
                    pancakeswap::v3::reserves_call_data::ReservesCallData::new(
                        address,
                        token0,
                        token1,
                        pool_info.fee,
                    ),
                ))
            }
        }
    }
}
