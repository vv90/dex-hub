use crate::blockchain::Blockchain;
use crate::subgraph::{SubgraphConfig, SubgraphQueryParams};
use crate::tokens::Token;
use crate::tokens::TokenAddress;
use crate::uniswap_internal::v4::pool::{Fee, Pool, PoolId};
use crate::utils::u32_from_str;
use alloy::primitives::{Address, FixedBytes};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TokenData {
    #[serde(rename = "id")]
    pub address: Address,
    #[serde(deserialize_with = "u32_from_str")]
    pub decimals: u32,
    pub symbol: String,
    // pub name: String,
    // #[serde(rename = "totalSupply")]
    // pub total_supply: u128,
    // #[serde(rename = "totalValueLocked")]
    // pub total_value_locked: Decimal,
    // #[serde(rename = "totalValueLockedUSD")]
    // pub total_value_locked_usd: Decimal,
    // #[serde(rename = "totalValueLockedUSDUntracked")]
    // pub total_value_locked_usd_untracked: Decimal,
    // pub volume: Decimal,
    // #[serde(rename = "volumeUSD")]
    // pub volume_usd: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PoolData {
    pub id: FixedBytes<32>,
    #[serde(rename = "feeTier", deserialize_with = "u32_from_str")]
    // #[serde(rename = "feeTier")]
    pub fee: u32,
    // #[serde(rename = "sqrtPrice")]
    // pub sqrt_price_x96: U160,
    // pub liquidity: U160,
    // #[serde(deserialize_with = "tick_from_str")]
    // pub tick: i32,
    #[serde(rename = "tickSpacing", deserialize_with = "u32_from_str")]
    pub tick_spacing: u32,
    // pub hooks: Address,
    pub token0: TokenData,
    pub token1: TokenData,
    // #[serde(rename = "totalValueLockedToken0")]
    // pub total_value_locked_token_0: Decimal,
    // #[serde(rename = "totalValueLockedToken1")]
    // pub total_value_locked_token_1: Decimal,
    // #[serde(rename = "totalValueLockedUSD")]
    // pub total_value_locked_usd: Decimal,
    // #[serde(rename = "totalValueLockedUSDUntracked")]
    // pub total_value_locked_untracked: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubgraphResponse {
    pools: Vec<PoolData>,
}

fn map_pools(data: SubgraphResponse, blockchain: Blockchain) -> Vec<Pool> {
    data.pools
        .into_iter()
        .map(|pool_data| Pool {
            pool_id: PoolId(pool_data.id, blockchain),
            fee: Fee(pool_data.fee),
            tick_spacing: pool_data.tick_spacing,
            token0: Token {
                address: TokenAddress(pool_data.token0.address, blockchain),
                symbol: pool_data.token0.symbol,
                decimals: pool_data.token0.decimals,
            },
            token1: Token {
                address: TokenAddress(pool_data.token1.address, blockchain),
                symbol: pool_data.token1.symbol,
                decimals: pool_data.token1.decimals,
            },
        })
        .collect()
}

const QUERY: &str =
    "{ id feeTier tickSpacing token0 { id symbol decimals } token1 { id symbol decimals } }";

pub fn format_query(
    SubgraphQueryParams {
        limit,
        skip,
        min_value,
    }: SubgraphQueryParams,
) -> String {
    format!(
        "{{ pools(first:{}, skip: {}, where: {{ totalValueLockedUSD_gt: {}, hooks: \\\"0x0000000000000000000000000000000000000000\\\" }}, orderBy: totalValueLockedUSD, orderDirection: desc) {} }}",
        limit, skip, min_value, QUERY
    )
}

pub const ETHEREUM: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V4_SUBGRAPH_ETH_URL"),
    subgraph_name: "ethereum/uniswap/v4",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::Ethereum),
};

pub const BSC: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V4_SUBGRAPH_BSC_URL"),
    subgraph_name: "bsc/uniswap/v4",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::BSC),
};

pub const ARBITRUM: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V4_SUBGRAPH_ARBITRUM_URL"),
    subgraph_name: "arbitrum/uniswap/v4",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::Arbitrum),
};
