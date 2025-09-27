use crate::{Blockchain, pancakeswap, uniswap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PoolId {
    UniswapV2(uniswap::v2::PoolAddress),
    UniswapV3(uniswap::v3::PoolAddress),
    UniswapV4(uniswap::v4::PoolId),
    PancakeSwap(pancakeswap::v3::PoolAddress),
}

impl PoolId {
    pub fn blockchain(&self) -> Blockchain {
        match self {
            PoolId::UniswapV2(uniswap::v2::PoolAddress(_, blockchain)) => *blockchain,
            PoolId::UniswapV3(uniswap::v3::PoolAddress(_, blockchain)) => *blockchain,
            PoolId::UniswapV4(uniswap::v4::PoolId(_, blockchain)) => *blockchain,
            PoolId::PancakeSwap(pancakeswap::v3::PoolAddress(_, blockchain)) => *blockchain,
        }
    }
}
