use crate::{evm_network::Ethereum, rpc::client::init_client};

mod blockchain;
mod evm_network;
mod multicall;
mod pool_id;
mod rpc;
mod subgraph;
mod tokens;
mod uniswap;
mod utils;

use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub async fn run_chains() -> Result<()> {
    const MIN_TVL: Decimal = dec!(1000.0);
    let pools = uniswap::v3::subgraph::ETHEREUM.query_pools(MIN_TVL).await?;

    let client_eth = init_client::<Ethereum>().await?;

    let block_number_eth = client_eth.get_block_number().await?;

    println!("eth block number: {}", block_number_eth);

    Ok(())
}
