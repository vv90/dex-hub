use alloy::primitives::Address;
use serde::Deserialize;

use crate::{blockchain::Blockchain, tokens::Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PoolAddress(pub(crate) Address, pub Blockchain);

pub struct Pool {
    pub address: PoolAddress,
    pub token0: Token,
    pub token1: Token,
}
