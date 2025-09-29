use alloy::primitives::Address;

use crate::{blockchain::Blockchain, tokens::Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PoolAddress(pub(crate) Address, pub Blockchain);

impl PoolAddress {
    pub fn blockchain(&self) -> Blockchain {
        self.1
    }
}

pub struct PoolInfo {
    pub token0: Token,
    pub token1: Token,
}

pub struct Pool {
    pub address: PoolAddress,
    pub info: PoolInfo,
}
