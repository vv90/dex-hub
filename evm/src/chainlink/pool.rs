use alloy::primitives::Address;

use crate::Blockchain;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoolAddress(pub(crate) Address, pub Blockchain);
