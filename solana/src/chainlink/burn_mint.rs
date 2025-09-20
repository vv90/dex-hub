use solana_sdk::pubkey::Pubkey;

use crate::chainlink::{base_chain::BaseChain, base_config::BaseConfig};

pub struct PoolConfig {
    pub version: u8,
    pub self_served_allowed: bool,
    pub router: Pubkey,
    pub rmn_remote: Pubkey,
}

pub struct State {
    pub version: u8,
    pub config: BaseConfig,
}

pub struct ChainConfig {
    pub base: BaseChain,
}
