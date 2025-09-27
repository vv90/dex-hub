mod blockchain;
mod evm_network;
mod multicall;

#[path = "pancakeswap/mod.rs"]
mod pancakeswap_internal;

mod reserves;
mod rpc;
mod subgraph;

#[path = "uniswap/mod.rs"]
mod uniswap_internal;

mod utils;
mod virtual_reserves;

pub mod chainlink;
mod pool_id;
pub mod state_manager;
pub mod tokens;

// pub const BLACKLIST: LazyLock<HashSet<TokenAddress>> = tokens::BLACKLIST;

pub use blockchain::Blockchain;
pub use pool_id::PoolId;

pub mod uniswap {
    pub mod v2 {
        pub use crate::uniswap_internal::v2::pool::*;
        pub use crate::uniswap_internal::v2::subgraph::get_pools;
    }
    pub mod v3 {
        pub use crate::uniswap_internal::v3::pool::*;
        pub use crate::uniswap_internal::v3::subgraph::get_pools;
    }
    pub mod v4 {
        pub use crate::uniswap_internal::v4::pool::*;
        pub use crate::uniswap_internal::v4::subgraph::get_pools;
    }
}

pub mod pancakeswap {
    pub mod v3 {
        pub use crate::pancakeswap_internal::v3::pool::*;
        pub use crate::pancakeswap_internal::v3::subgraph::get_pools;
    }
}
