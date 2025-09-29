use crate::blockchain::Blockchain;
use crate::pancakeswap::v3::PoolInfo;
use crate::pancakeswap_internal::v3::pool::Pool;
use crate::subgraph::{SubgraphConfig, SubgraphQueryParams};
use crate::tokens::{Token, TokenAddress};
use crate::{
    pancakeswap_internal::v3::pool::{Fee, PoolAddress, fee_from_int},
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
}

#[derive(Debug, Clone, Deserialize)]
struct PoolData {
    #[serde(rename = "id")]
    pub address: Address,
    #[serde(rename = "feeTier", deserialize_with = "fee_from_int_str")]
    pub fee: Fee,
    pub token0: TokenData,
    pub token1: TokenData,
    // #[serde(rename = "totalValueLockedUSD")]
    // pub total_value_locked_usd: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
struct SubgraphResponse {
    pools: Vec<PoolData>,
}

fn map_pools(data: SubgraphResponse, blockchain: Blockchain) -> Vec<Pool> {
    data.pools
        .into_iter()
        .map(|pool_data| Pool {
            address: PoolAddress(pool_data.address, blockchain),
            info: PoolInfo {
                fee: pool_data.fee,
                token0: Token {
                    address: TokenAddress(pool_data.token0.address, blockchain),
                    decimals: pool_data.token0.decimals,
                    symbol: pool_data.token0.symbol,
                },
                token1: Token {
                    address: TokenAddress(pool_data.token1.address, blockchain),
                    decimals: pool_data.token1.decimals,
                    symbol: pool_data.token1.symbol,
                },
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
        "{{ pools (first:{}, skip:{}, orderBy:totalValueLockedUSD, orderDirection:desc, where:{{ totalValueLockedUSD_gt:{} }}) {} }}",
        limit, skip, min_value, QUERY
    )
}

const ETHEREUM: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("PANCAKESWAP_V3_SUBGRAPH_ETH_URL"),
    subgraph_name: "ethereum/pancakeswap/v3",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::Ethereum),
};

const BSC: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("PANCAKESWAP_V3_SUBGRAPH_BSC_URL"),
    subgraph_name: "bsc/pancakeswap/v3",
    format_query,
    map_pools: |data| map_pools(data, Blockchain::BSC),
};

const ARBITRUM: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("PANCAKESWAP_V3_SUBGRAPH_ARBITRUM_URL"),
    subgraph_name: "arbitrum/pancakeswap/v3",
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
