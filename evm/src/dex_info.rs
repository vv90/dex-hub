use std::collections::HashMap;

use crate::{
    PoolId, pancakeswap_internal as pancakeswap,
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

impl DexInfo {
    pub fn lookup_pool_tokens(&self, pool_id: PoolId) -> Option<(TokenAddress, TokenAddress)> {
        match pool_id {
            PoolId::UniswapV2(pool_address) => self
                .uniswap_v2_pools
                .get(&pool_address)
                .map(|pool| (pool.token0, pool.token1)),
            PoolId::UniswapV3(pool_address) => self
                .uniswap_v3_pools
                .get(&pool_address)
                .map(|pool| (pool.token0, pool.token1)),
            PoolId::UniswapV4(pool_id) => self
                .uniswap_v4_pools
                .get(&pool_id)
                .map(|pool| (pool.token0, pool.token1)),
            PoolId::PancakeSwap(pool_address) => self
                .pancakeswap_pools
                .get(&pool_address)
                .map(|pool| (pool.token0, pool.token1)),
        }
    }
}
