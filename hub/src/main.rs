use std::{collections::HashSet, sync::Arc};

use anyhow::Result;
use tokio::sync::mpsc;

use crate::pools::{PoolId, TokensConnectionType, collect_pools};

mod graph;
mod pools;
mod tokens;

async fn get_reserves() -> Result<()> {
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let (pool_information, tokens_graph) = collect_pools().await?;

    let evm_pool_ids = tokens_graph
        .adjacency_ids()
        .filter_map(|adj_id| match adj_id {
            TokensConnectionType::Swap(PoolId::Evm(pool_id)) => Some(*pool_id),
            _ => None,
        })
        .collect::<HashSet<evm::PoolId>>();

    println!("EVM pools size: {}", evm_pool_ids.len());

    let (sender, mut receiver) =
        mpsc::unbounded_channel::<Vec<(evm::PoolId, evm::VirtualReserves)>>();
    let state_manager = evm::StateManager::from_pools(&evm_pool_ids);
    let (handle, initial_reserves) = state_manager
        .subscribe_reserves(
            sender,
            Arc::new(pool_information.tokens),
            Arc::new(pool_information.uniswap_v2_pools),
            Arc::new(pool_information.uniswap_v3_pools),
            Arc::new(pool_information.uniswap_v4_pools),
            Arc::new(pool_information.pancakeswap_pools),
        )
        .await?;

    println!("Initial reserves size: {}", initial_reserves.len());

    while let Some(update) = receiver.recv().await {
        println!("{} pools updated", update.len());
    }
    // let r = evm::state_manager::StateManager::init(ethereum_pool_ids, evm::Blockchain::Ethereum)
    //     .await?;

    // let (sender, mut receiver) = mpsc::channel::<evm::uniswap::v3::PoolAddress>(100);

    // let _ = futures_util::future::try_join3(
    //     evm::uniswap::v3::subscribe_pool_updates(sender.clone(), evm::Blockchain::Ethereum),
    //     evm::uniswap::v3::subscribe_pool_updates(sender.clone(), evm::Blockchain::Arbitrum),
    //     evm::uniswap::v3::subscribe_pool_updates(sender.clone(), evm::Blockchain::BSC),
    // )
    // .await?;

    // while let Some(pool_address) = receiver.recv().await {
    //     let pool_id = PoolId::UniswapV3(pool_address);
    //     if pool_ids.contains(&pool_id) {
    //         println!("{:?}", pool_address);
    //     }
    // }
    let _ = handle.await??;
    Ok(())
}
