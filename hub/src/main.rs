use std::{collections::HashSet, sync::Arc, time::Duration};

use anyhow::Result;

use crate::pools::{PoolId, TokensConnectionType, collect_pools};

mod graph;
mod pools;
mod tokens;

#[tokio::main]
async fn main() -> Result<()> {
    let (dex_info, tokens_graph) = collect_pools().await?;

    let dex_info = Arc::new(dex_info);

    let evm_pool_ids = tokens_graph
        .adjacency_ids()
        .filter_map(|adj_id| match adj_id {
            TokensConnectionType::Swap(PoolId::Evm(pool_id)) => Some(*pool_id),
            _ => None,
        })
        .collect::<HashSet<evm::PoolId>>();

    println!("EVM pools size: {}", evm_pool_ids.len());

    let (mut state_manager, initial_reserves) =
        evm::StateManager::init(&evm_pool_ids, dex_info).await?;

    println!("Initial reserves size: {}", initial_reserves.len());

    loop {
        let (state_manager_, reserves) = state_manager.get_updated_reserves().await?;
        state_manager = state_manager_;

        println!(
            "Updated reserves eth: {}, bsc: {}, arb: {}",
            reserves
                .iter()
                .filter(|(id, _)| id.blockchain() == evm::Blockchain::Ethereum)
                .count(),
            reserves
                .iter()
                .filter(|(id, _)| id.blockchain() == evm::Blockchain::BSC)
                .count(),
            reserves
                .iter()
                .filter(|(id, _)| id.blockchain() == evm::Blockchain::Arbitrum)
                .count()
        );
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
