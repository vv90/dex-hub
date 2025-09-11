use solana_sdk::pubkey::Pubkey;

use crate::tokens::Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoolAddress(pub(crate) Pubkey);

pub struct Fee(u32);

pub struct Pool {
    pub address: PoolAddress,
    pub token0: Token,
    pub token1: Token,
    pub fee: Fee,
    pub tick_spacing: u32,
}
