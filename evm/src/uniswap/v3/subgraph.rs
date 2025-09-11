use crate::blockchain::Blockchain;
use crate::subgraph::{SubgraphConfig, SubgraphQueryParams};
use crate::tokens::{Token, TokenAddress};
use crate::{
    uniswap_internal::v3::pool::{Fee, Pool, PoolAddress, fee_from_int},
    utils::u32_from_str,
};
use alloy::primitives::Address;
use anyhow::Result;
use rust_decimal::Decimal;
use serde::Deserialize;

fn fee_from_int_str<'de, D>(deserializer: D) -> Result<Fee, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let fee_amount = serde_json::from_str(s.as_str()).map_err(serde::de::Error::custom)?;
    fee_from_int(fee_amount).map_err(|e| serde::de::Error::custom(e))
}

#[derive(Debug, Clone, Deserialize)]
struct TokenData {
    #[serde(rename = "id")]
    pub address: Address,
    #[serde(deserialize_with = "u32_from_str")]
    pub decimals: u32,
    pub symbol: String,
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
struct PoolData {
    #[serde(rename = "id")]
    pub address: Address,
    #[serde(rename = "feeTier", deserialize_with = "fee_from_int_str")]
    // #[serde(rename = "feeTier")]
    pub fee: Fee,
    // #[serde(rename = "sqrtPrice")]
    // pub sqrt_price_x96: U160,
    // pub liquidity: U160,
    // #[serde(deserialize_with = "tick_from_str")]
    // pub tick: i32,
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
struct SubgraphResponse {
    pools: Vec<PoolData>,
}

fn map_pools(data: SubgraphResponse, blockchain: Blockchain) -> Vec<Pool> {
    data.pools
        .into_iter()
        .map(|pool| Pool {
            address: PoolAddress(pool.address, blockchain),
            fee: pool.fee,
            token0: Token {
                address: TokenAddress(pool.token0.address, blockchain),
                decimals: pool.token0.decimals,
                symbol: pool.token0.symbol,
            },
            token1: Token {
                address: TokenAddress(pool.token1.address, blockchain),
                decimals: pool.token1.decimals,
                symbol: pool.token1.symbol,
            },
        })
        .collect()
}

const QUERY: &str = "{ id feeTier token0 { id decimals symbol } token1 { id decimals symbol } }";

fn format_query(
    SubgraphQueryParams {
        limit,
        skip,
        min_value,
    }: SubgraphQueryParams,
) -> String {
    format!(
        "{{ pools (first: {}, skip:{}, where: {{ totalValueLockedUSD_gt: {} }}, orderBy: totalValueLockedUSD, orderDirection: desc) {} }}",
        limit, skip, min_value, QUERY
    )
}

const ETHEREUM: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V3_SUBGRAPH_ETH_URL"),
    subgraph_name: "ethereum/uniswap/v3",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::Ethereum),
};

const BSC: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V3_SUBGRAPH_BSC_URL"),
    subgraph_name: "bsc/uniswap/v3",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::BSC),
};

const ARBITRUM: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V3_SUBGRAPH_ARBITRUM_URL"),
    subgraph_name: "arbitrum/uniswap/v3",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::Arbitrum),
};

pub async fn get_pools(blockchain: Blockchain, min_value: Decimal) -> Result<Vec<Pool>> {
    match blockchain {
        Blockchain::Ethereum => ETHEREUM.query_pools(min_value).await,
        Blockchain::BSC => BSC.query_pools(min_value).await,
        Blockchain::Arbitrum => ARBITRUM.query_pools(min_value).await,
    }
}
