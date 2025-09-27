use std::collections::HashSet;

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
    let tokens_graph = collect_pools().await?;

    let pool_ids = tokens_graph
        .adjacency_ids()
        .into_iter()
        .filter_map(|adj_id| match adj_id {
            TokensConnectionType::Swap(pool_id) => Some(pool_id),
            _ => None,
        })
        .collect::<HashSet<&PoolId>>();

    let ethereum_pool_ids = pool_ids
        .iter()
        .filter_map(|pool_id| match pool_id {
            PoolId::Evm(evm_pool_id) => {
                if evm_pool_id.blockchain() == evm::Blockchain::Ethereum {
                    Some(*evm_pool_id)
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect::<HashSet<_>>();

    // let r = evm::state_manager::StateManager::init(ethereum_pool_ids, evm::Blockchain::Ethereum)
    //     .await?;

    let (sender, mut receiver) = mpsc::channel::<evm::uniswap::v3::PoolAddress>(100);

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

    Ok(())
}
