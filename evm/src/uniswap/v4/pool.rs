use alloy::primitives::FixedBytes;
use serde::Deserialize;

use crate::{blockchain::Blockchain, tokens::Token};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fee(pub u32);

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoolId(pub FixedBytes<32>, pub Blockchain);

pub struct Pool {
    pub pool_id: PoolId,
    pub fee: Fee,
    pub tick_spacing: u32,
    pub token0: Token,
    pub token1: Token,
}
