use anyhow::Result;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use solana_sdk::pubkey::Pubkey;

// const WHIRPOOLS_API_URL: &str = "https://api.orca.so/v2/solana/pools?sortBy=volume&sortDirection=desc&hasAdaptiveFee=false&size=100";
const WHIRPOOLS_API_URL: &str =
    "https://api.orca.so/v2/solana/pools?sortBy=volume&sortDirection=desc&minTvl=1000&size=1000";

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct TokenDto {
    #[serde_as(as = "DisplayFromStr")]
    pub address: Pubkey,
    pub decimals: u8,
    pub symbol: Option<String>,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct PoolDto {
    #[serde_as(as = "DisplayFromStr")]
    pub address: Pubkey,
    #[serde(rename = "feeRate")]
    pub fee: u32,
    #[serde(rename = "protocolFeeRate")]
    pub protocol_fee: u32,
    #[serde(rename = "tickSpacing")]
    pub tick_spacing: u32,
    #[serde(rename = "tokenA")]
    pub token0: TokenDto,
    #[serde(rename = "tokenB")]
    pub token1: TokenDto,
}

#[derive(Debug, Deserialize)]
struct PoolsResponseCursor {
    pub next: Option<String>,
    pub previous: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PoolsResponseMeta {
    cursor: PoolsResponseCursor,
}

#[derive(Debug, Deserialize)]
struct PoolsResponse {
    pub data: Vec<PoolDto>,
    pub meta: PoolsResponseMeta,
}

async fn get_pools_rec(next: String, mut pools: Vec<PoolDto>) -> Result<Vec<PoolDto>> {
    let url = format!("{}&next={}", WHIRPOOLS_API_URL, next);
    println!("{}", url);
    let response = reqwest::get(&url)
        .await?
        .error_for_status()?
        .json::<PoolsResponse>()
        .await?;

    println!("{}, {:?}", response.data.len(), response.meta.cursor);
    pools.extend(response.data);
    match response.meta.cursor.next {
        Some(next) => Box::pin(get_pools_rec(next, pools)).await,
        None => Ok(pools),
    }
}

pub async fn get_pools() -> Result<Vec<PoolDto>> {
    // println!("body = {body:?}");
    let pools = Vec::new();
    get_pools_rec(String::new(), pools).await
}
