use crate::{
    Blockchain, PoolId, blockchain::BlockchainNetwork, evm_network, multicall,
    pancakeswap_internal as pancakeswap, rpc::multicall_data::MulticallData,
    uniswap_internal as uniswap, virtual_reserves::VirtualReserves,
};
use alloy::primitives::{Address, Bytes, FixedBytes};
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

pub struct PoolReservesCalls {
    pub ethereum: Vec<ReservesCallData<evm_network::Ethereum>>,
    pub bsc: Vec<ReservesCallData<evm_network::BSC>>,
    pub arbitrum: Vec<ReservesCallData<evm_network::Arbitrum>>,
}

impl PoolReservesCalls {
    pub fn new() -> Self {
        Self {
            ethereum: Vec::new(),
            bsc: Vec::new(),
            arbitrum: Vec::new(),
        }
    }

    pub fn with_uniswap_v2_call(
        mut self,
        pool_address: &uniswap::v2::pool::PoolAddress,
        pool_info: &uniswap::v2::pool::PoolInfo,
    ) -> Result<Self> {
        match pool_address.blockchain() {
            Blockchain::Ethereum => {
                self.ethereum.push(ReservesCallData::UniswapV2(
                    uniswap::v2::reserves_call_data::ReservesCallData::create(
                        pool_address,
                        pool_info,
                    )?,
                ));
                Ok(self)
            }
            Blockchain::BSC => {
                self.bsc.push(ReservesCallData::UniswapV2(
                    uniswap::v2::reserves_call_data::ReservesCallData::create(
                        pool_address,
                        pool_info,
                    )?,
                ));
                Ok(self)
            }
            Blockchain::Arbitrum => {
                self.arbitrum.push(ReservesCallData::UniswapV2(
                    uniswap::v2::reserves_call_data::ReservesCallData::create(
                        pool_address,
                        pool_info,
                    )?,
                ));
                Ok(self)
            }
        }
    }

    pub fn with_uniswap_v3_call(
        mut self,
        pool_address: &uniswap::v3::pool::PoolAddress,
        pool_info: &uniswap::v3::pool::PoolInfo,
    ) -> Result<Self> {
        match pool_address.blockchain() {
            Blockchain::Ethereum => {
                self.ethereum.push(ReservesCallData::UniswapV3(
                    uniswap::v3::reserves_call_data::ReservesCallData::create(
                        pool_address,
                        pool_info,
                    )?,
                ));
                Ok(self)
            }
            Blockchain::BSC => {
                self.bsc.push(ReservesCallData::UniswapV3(
                    uniswap::v3::reserves_call_data::ReservesCallData::create(
                        pool_address,
                        pool_info,
                    )?,
                ));
                Ok(self)
            }
            Blockchain::Arbitrum => {
                self.arbitrum.push(ReservesCallData::UniswapV3(
                    uniswap::v3::reserves_call_data::ReservesCallData::create(
                        pool_address,
                        pool_info,
                    )?,
                ));
                Ok(self)
            }
        }
    }

    pub fn with_uniswap_v4_call(
        mut self,
        pool_id: &uniswap::v4::pool::PoolId,
        pool_info: &uniswap::v4::pool::PoolInfo,
    ) -> Result<Self> {
        match pool_id.blockchain() {
            Blockchain::Ethereum => {
                self.ethereum.push(ReservesCallData::UniswapV4(
                    uniswap::v4::reserves_call_data::ReservesCallData::create(pool_id, pool_info)?,
                ));
                Ok(self)
            }
            Blockchain::BSC => {
                self.bsc.push(ReservesCallData::UniswapV4(
                    uniswap::v4::reserves_call_data::ReservesCallData::create(pool_id, pool_info)?,
                ));
                Ok(self)
            }
            Blockchain::Arbitrum => {
                self.arbitrum.push(ReservesCallData::UniswapV4(
                    uniswap::v4::reserves_call_data::ReservesCallData::create(pool_id, pool_info)?,
                ));
                Ok(self)
            }
        }
    }

    pub fn with_pancakeswap_v3_call(
        mut self,
        pool_address: &pancakeswap::v3::pool::PoolAddress,
        pool_info: &pancakeswap::v3::pool::PoolInfo,
    ) -> Result<Self> {
        match pool_address.blockchain() {
            Blockchain::Ethereum => {
                self.ethereum.push(ReservesCallData::PancakeSwap(
                    pancakeswap::v3::reserves_call_data::ReservesCallData::create(
                        pool_address,
                        pool_info,
                    )?,
                ));
                Ok(self)
            }
            Blockchain::BSC => {
                self.bsc.push(ReservesCallData::PancakeSwap(
                    pancakeswap::v3::reserves_call_data::ReservesCallData::create(
                        pool_address,
                        pool_info,
                    )?,
                ));
                Ok(self)
            }
            Blockchain::Arbitrum => {
                self.arbitrum.push(ReservesCallData::PancakeSwap(
                    pancakeswap::v3::reserves_call_data::ReservesCallData::create(
                        pool_address,
                        pool_info,
                    )?,
                ));
                Ok(self)
            }
        }
    }
}
