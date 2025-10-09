use alloy::primitives::Address;

use crate::{blockchain::Blockchain, tokens::TokenAddress};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PoolAddress(pub(crate) Address, pub Blockchain);

impl PoolAddress {
    pub fn blockchain(&self) -> Blockchain {
        self.1
    }
}

pub struct PoolInfo {
    pub token0: TokenAddress,
    pub token1: TokenAddress,
}

pub struct Pool {
    pub address: PoolAddress,
    pub info: PoolInfo,
}
