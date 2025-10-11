use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use crate::blockchain::Blockchain;
use crate::pancakeswap::v3::PoolInfo;
use crate::pancakeswap_internal::v3::pool::Pool;
use crate::subgraph::{SubgraphConfig, SubgraphQueryParams};
use crate::tokens::{TokenAddress, TokenInfo};
use crate::{
    pancakeswap_internal::v3::pool::{Fee, PoolAddress, fee_from_int},
    utils::u32_from_str,
};
use alloy::primitives::{Address, address};
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

fn map_pools(
    blockchain: Blockchain,
    tokens_map: HashMap<TokenAddress, TokenInfo>,
    data: SubgraphResponse,
) -> (Vec<Pool>, HashMap<TokenAddress, TokenInfo>) {
    data.pools.into_iter().fold(
        (Vec::new(), tokens_map),
        |(mut pools, mut tokens), pool_data| {
            let token0_address = TokenAddress(pool_data.token0.address, blockchain);
            let token1_address = TokenAddress(pool_data.token1.address, blockchain);
            let pool = Pool {
                address: PoolAddress(pool_data.address, blockchain),
                info: PoolInfo {
                    token0: token0_address,
                    token1: token1_address,
                    fee: pool_data.fee,
                },
            };

            // TODO: Temporary workaround for the reserves value overflow
            // in rare cases, a combination of small decimal places and large reserves
            // can cause an overflow when converting reserves into decimal
            // a proper solution would be to adjust decimal places (if needed) for each token
            if !BLACKLIST.contains(&pool.address) {
                pools.push(pool);
                tokens.entry(token0_address).or_insert_with(|| TokenInfo {
                    decimals: pool_data.token0.decimals,
                    symbol: pool_data.token0.symbol,
                });
                tokens.entry(token1_address).or_insert_with(|| TokenInfo {
                    decimals: pool_data.token1.decimals,
                    symbol: pool_data.token1.symbol,
                });
            }

            (pools, tokens)
        },
    )
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
    map_pools: |tokens, data| map_pools(Blockchain::Ethereum, tokens, data),
};

const BSC: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("PANCAKESWAP_V3_SUBGRAPH_BSC_URL"),
    subgraph_name: "bsc/pancakeswap/v3",
    format_query,
    map_pools: |tokens, data| map_pools(Blockchain::BSC, tokens, data),
};

const ARBITRUM: SubgraphConfig<SubgraphResponse, Pool> = SubgraphConfig {
    subgraph_url: env!("PANCAKESWAP_V3_SUBGRAPH_ARBITRUM_URL"),
    subgraph_name: "arbitrum/pancakeswap/v3",
    format_query,
    map_pools: |tokens, data| map_pools(Blockchain::Arbitrum, tokens, data),
};

pub async fn get_pools(
    blockchain: Blockchain,
    min_value: Decimal,
) -> Result<(Vec<Pool>, HashMap<TokenAddress, TokenInfo>)> {
    match blockchain {
        Blockchain::Ethereum => ETHEREUM.query_pools(min_value).await,
        Blockchain::BSC => BSC.query_pools(min_value).await,
        Blockchain::Arbitrum => ARBITRUM.query_pools(min_value).await,
    }
}

const BLACKLIST: LazyLock<HashSet<PoolAddress>> = LazyLock::new(|| {
    HashSet::from([PoolAddress(
        address!("0x7837afeD5d6176873F80F550FaC452dBDDBe03A9"),
        Blockchain::BSC,
    )])
});
