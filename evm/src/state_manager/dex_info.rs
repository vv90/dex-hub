use std::collections::HashMap;

use crate::{
    pancakeswap_internal as pancakeswap,
    tokens::{TokenAddress, TokenInfo},
    uniswap_internal as uniswap,
};

pub struct DexInfo {
    pub tokens: HashMap<TokenAddress, TokenInfo>,
    pub uniswap_v2_pools: HashMap<uniswap::v2::pool::PoolAddress, uniswap::v2::pool::PoolInfo>,
    pub uniswap_v3_pools: HashMap<uniswap::v3::pool::PoolAddress, uniswap::v3::pool::PoolInfo>,
    pub uniswap_v4_pools: HashMap<uniswap::v4::pool::PoolId, uniswap::v4::pool::PoolInfo>,
    pub pancakeswap_pools:
        HashMap<pancakeswap::v3::pool::PoolAddress, pancakeswap::v3::pool::PoolInfo>,
}
