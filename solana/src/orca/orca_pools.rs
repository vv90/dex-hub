use anyhow::Result;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use solana_sdk::pubkey::Pubkey;

use crate::{
    orca::{Fee, Pool, PoolAddress},
    tokens::{Token, TokenAddress},
};

// const WHIRPOOLS_API_URL: &str = "https://api.orca.so/v2/solana/pools?sortBy=volume&sortDirection=desc&hasAdaptiveFee=false&size=100";
const WHIRPOOLS_API_URL: &str =
    "https://api.orca.so/v2/solana/pools?sortBy=volume&sortDirection=desc&size=1000";

#[serde_as]
#[derive(Debug, Deserialize)]
struct TokenDto {
    #[serde_as(as = "DisplayFromStr")]
    address: Pubkey,
    decimals: u8,
    symbol: Option<String>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
struct PoolDto {
    #[serde_as(as = "DisplayFromStr")]
    address: Pubkey,
    #[serde(rename = "feeRate")]
    fee: u32,
    // #[serde(rename = "protocolFeeRate")]
    // protocol_fee: u32,
    #[serde(rename = "tickSpacing")]
    tick_spacing: u32,
    #[serde(rename = "tokenA")]
    token0: TokenDto,
    #[serde(rename = "tokenB")]
    token1: TokenDto,
}

#[derive(Debug, Deserialize)]
struct PoolsResponseCursor {
    next: Option<String>,
    // previous: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PoolsResponseMeta {
    cursor: PoolsResponseCursor,
}

#[derive(Debug, Deserialize)]
struct PoolsResponse {
    data: Vec<PoolDto>,
    meta: PoolsResponseMeta,
}

async fn get_pools_rec(
    next: String,
    min_value: i32,
    mut pools: Vec<PoolDto>,
) -> Result<Vec<PoolDto>> {
    let url = format!("{}&minTvl={}&next={}", WHIRPOOLS_API_URL, min_value, next);
    println!("{}", url);
    let response = reqwest::get(&url)
        .await?
        .error_for_status()?
        .json::<PoolsResponse>()
        .await?;

    println!("{}, {:?}", response.data.len(), response.meta.cursor);
    pools.extend(response.data);
    match response.meta.cursor.next {
        Some(next) => Box::pin(get_pools_rec(next, min_value, pools)).await,
        None => Ok(pools),
    }
}

pub async fn get_pools(min_value: i32) -> Result<Vec<Pool>> {
    let pools = get_pools_rec(String::new(), min_value, Vec::new()).await?;
    Ok(pools
        .into_iter()
        .map(|dto| Pool {
            address: PoolAddress(dto.address),
            token0: Token {
                address: TokenAddress(dto.token0.address),
                decimals: dto.token0.decimals,
                symbol: dto.token0.symbol,
            },
            token1: Token {
                address: TokenAddress(dto.token1.address),
                decimals: dto.token1.decimals,
                symbol: dto.token1.symbol,
            },
            fee: Fee(dto.fee),
            tick_spacing: dto.tick_spacing,
        })
        .collect())
}
