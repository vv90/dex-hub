use std::collections::HashMap;

use crate::blockchain::Blockchain;
use crate::subgraph::{SubgraphConfig, SubgraphQueryParams};
use crate::tokens::{TokenAddress, TokenInfo};
use crate::uniswap::v2::PoolInfo;
use crate::uniswap_internal::v2::pool::{Pool, PoolAddress};
use crate::utils::u32_from_str;
use alloy::primitives::Address;
use anyhow::Result;
use rust_decimal::Decimal;
use serde::Deserialize;

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
struct SubgraphResponse {
    pairs: Vec<PoolData>,
}

fn map_pools(
    blockchain: Blockchain,
    tokens_map: HashMap<TokenAddress, TokenInfo>,
    data: SubgraphResponse,
) -> (Vec<Pool>, HashMap<TokenAddress, TokenInfo>) {
    data.pairs.into_iter().fold(
        (Vec::<Pool>::new(), tokens_map),
        |(mut pools, mut tokens), pool_data| {
            let token0_address = TokenAddress(pool_data.token0.address, blockchain);
            let token1_address = TokenAddress(pool_data.token1.address, blockchain);
            let pool = Pool {
                address: PoolAddress(pool_data.address, blockchain),
                info: PoolInfo {
                    token0: token0_address,
                    token1: token1_address,
                },
            };

            pools.push(pool);
            tokens.entry(token0_address).or_insert_with(|| TokenInfo {
                decimals: pool_data.token0.decimals,
                symbol: pool_data.token0.symbol,
            });
            tokens.entry(token1_address).or_insert_with(|| TokenInfo {
                decimals: pool_data.token1.decimals,
                symbol: pool_data.token1.symbol,
            });

            (pools, tokens)
        },
    )
}

const ETHEREUM_SUBGRAPH: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V2_SUBGRAPH_ETH_URL"),
    subgraph_name: "ethereum/uniswap/v2",
    format_query,
    map_pools: |tokens, data| map_pools(Blockchain::Ethereum, tokens, data),
};

const BSC_SUBGRAPH: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V2_SUBGRAPH_BSC_URL"),
    subgraph_name: "bsc/uniswap/v2",
    format_query,
    map_pools: |tokens, data| map_pools(Blockchain::BSC, tokens, data),
};

const ARBITRUM_SUBGRAPH: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("UNISWAP_V2_SUBGRAPH_ARBITRUM_URL"),
    subgraph_name: "arbitrum/uniswap/v2",
    format_query,
    map_pools: |tokens, data| map_pools(Blockchain::Arbitrum, tokens, data),
};

pub async fn get_pools(
    blockchain: Blockchain,
    min_value: Decimal,
) -> Result<(Vec<Pool>, HashMap<TokenAddress, TokenInfo>)> {
    match blockchain {
        Blockchain::Ethereum => ETHEREUM_SUBGRAPH.query_pools(min_value).await,
        Blockchain::BSC => BSC_SUBGRAPH.query_pools(min_value).await,
        Blockchain::Arbitrum => ARBITRUM_SUBGRAPH.query_pools(min_value).await,
    }
}
