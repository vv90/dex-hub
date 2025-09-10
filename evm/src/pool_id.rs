use alloy::primitives::{Address, FixedBytes};

use crate::blockchain::Blockchain;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PoolId {
    UniswapV2(Address, Blockchain),
    UniswapV3(Address, Blockchain),
    UniswapV4(FixedBytes<32>, Blockchain),
    PancakeSwapV3(Address, Blockchain),
}
