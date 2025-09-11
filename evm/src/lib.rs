use crate::{evm_network::Ethereum, rpc::client::init_client};

mod blockchain;
mod evm_network;
mod multicall;

#[path = "pancakeswap/mod.rs"]
mod pancakeswap_internal;

mod pool_id;
mod reserves;
mod rpc;
mod subgraph;

#[path = "uniswap/mod.rs"]
mod uniswap_internal;

mod utils;
mod virtual_reserves;

pub mod tokens;

use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// pub const BLACKLIST: LazyLock<HashSet<TokenAddress>> = tokens::BLACKLIST;

pub use blockchain::Blockchain;

pub mod uniswap {
    pub mod v2 {
        pub use crate::uniswap_internal::v2::pool::PoolAddress;
    }
    pub mod v3 {
        pub use crate::uniswap_internal::v3::pool::PoolAddress;
    }
    pub mod v4 {
        pub use crate::uniswap_internal::v4::pool::PoolId;
    }
}

pub mod pancakeswap {
    pub mod v3 {
        pub use crate::pancakeswap_internal::v3::pool::PoolAddress;
    }
}

pub async fn run_chains() -> Result<()> {
    const MIN_TVL: Decimal = dec!(1000.0);
    let pools = uniswap_internal::v3::subgraph::ETHEREUM
        .query_pools(MIN_TVL)
        .await?;

    let client_eth = init_client::<Ethereum>().await?;

    let block_number_eth = client_eth.get_block_number().await?;

    println!("eth block number: {}", block_number_eth);

    Ok(())
}
