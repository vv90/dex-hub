use crate::blockchain::Blockchain;
use crate::subgraph::{SubgraphConfig, SubgraphQueryParams};
use crate::tokens::{Token, TokenAddress};
use crate::uniswap_internal::v2::pool::{Pool, PoolAddress};
use crate::utils::u32_from_str;
use alloy::primitives::Address;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TokenData {
    #[serde(rename = "id")]
    pub address: Address,
    #[serde(deserialize_with = "u32_from_str")]
    pub decimals: u32,
    pub symbol: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PoolData {
    #[serde(rename = "id")]
    pub address: Address,
    #[serde(rename = "token0")]
    pub token0: TokenData,
    #[serde(rename = "token1")]
    pub token1: TokenData,
    // pub reserve0: Decimal,
    // pub reserve1: Decimal,
    // #[serde(rename = "reserveUSD")]
    // pub reserve_usd: Decimal,
}

const QUERY: &str = "{ id token0 { id symbol decimals } token1 { id symbol decimals } }";

fn format_query(
    SubgraphQueryParams {
        limit,
        skip,
        min_value,
    }: SubgraphQueryParams,
) -> String {
    format!(
        "{{ pairs (first: {}, skip: {}, orderBy: reserveUSD, orderDirection: desc, where:{{reserveUSD_lt:9223372036854775807, reserveUSD_gt:{}}}) {} }}",
        limit, skip, min_value, QUERY
    )
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubgraphResponse {
    pairs: Vec<PoolData>,
}

fn map_pools(data: SubgraphResponse, blockchain: Blockchain) -> Vec<Pool> {
    data.pairs
        .into_iter()
        .map(|pool_data| Pool {
            address: PoolAddress(pool_data.address, blockchain),
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

pub const ETHEREUM: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V2_SUBGRAPH_ETH_URL"),
    subgraph_name: "ethereum/uniswap/v2",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::Ethereum),
};

pub const BSC: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V2_SUBGRAPH_BSC_URL"),
    subgraph_name: "bsc/uniswap/v2",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::BSC),
};

pub const ARBITRUM: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V2_SUBGRAPH_ARBITRUM_URL"),
    subgraph_name: "arbitrum/uniswap/v2",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::Arbitrum),
};
