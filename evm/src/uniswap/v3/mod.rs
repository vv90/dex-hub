pub mod contract;
pub mod pool;
pub mod pool_state;
pub mod pool_state_call_data;
pub mod reserves;
pub mod reserves_call_data;
pub mod router;
pub mod subgraph;

#[cfg_attr(feature = "testnet", path = "deployments/testnet.rs")]
#[cfg_attr(not(feature = "testnet"), path = "deployments/mainnet.rs")]
pub mod deployments;
