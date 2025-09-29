use alloy::primitives::FixedBytes;
use serde::Deserialize;

use crate::{blockchain::Blockchain, tokens::Token};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fee(pub u32);

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoolId(pub FixedBytes<32>, pub Blockchain);

impl PoolId {
    pub fn blockchain(&self) -> Blockchain {
        self.1
    }
}

pub struct PoolInfo {
    pub fee: Fee,
    pub tick_spacing: u32,
    pub token0: Token,
    pub token1: Token,
}

pub struct Pool {
    pub id: PoolId,
    pub info: PoolInfo,
}
