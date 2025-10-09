use crate::{
    blockchain::BlockchainNetwork, multicall, pancakeswap_internal as pancakeswap, pool_id::PoolId,
    rpc::multicall_data::MulticallData, uniswap_internal as uniswap,
    virtual_reserves::VirtualReserves,
};
use alloy::primitives::Bytes;
use anyhow::Result;

pub enum ReservesCallData<B: BlockchainNetwork> {
    UniswapV2(uniswap::v2::reserves_call_data::ReservesCallData<B>),
    UniswapV3(uniswap::v3::reserves_call_data::ReservesCallData<B>),
    UniswapV4(uniswap::v4::reserves_call_data::ReservesCallData<B>),
    PancakeSwap(pancakeswap::v3::reserves_call_data::ReservesCallData<B>),
}

impl<B: BlockchainNetwork> MulticallData<B> for ReservesCallData<B> {
    type Calls = Vec<multicall::Multicall3::Call>;
    type Output = (PoolId, VirtualReserves);

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
            ReservesCallData::UniswapV2(data) => data
                .decode_output(response)
                .map(|(address, reserves)| (PoolId::UniswapV2(address), reserves)),
            ReservesCallData::UniswapV3(data) => data
                .decode_output(response)
                .map(|(address, reserves)| (PoolId::UniswapV3(address), reserves)),
            ReservesCallData::UniswapV4(data) => data
                .decode_output(response)
                .map(|(id, reserves)| (PoolId::UniswapV4(id), reserves)),
            ReservesCallData::PancakeSwap(data) => data
                .decode_output(response)
                .map(|(address, reserves)| (PoolId::PancakeSwap(address), reserves)),
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
