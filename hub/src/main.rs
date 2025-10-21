use std::{
    collections::HashSet,
    fmt::format,
    fs::{self, File},
    io::Write,
    path::Path,
    sync::Arc,
};

use anyhow::{Result, anyhow};
use petgraph::{
    Graph,
    dot::{self, Dot},
};
use rust_decimal::prelude::*;

use crate::{
    pools::{Bridge, PoolId, TokensConnectionType, collect_pools},
    tokens::TokenId,
};

mod graph;
mod pools;
mod tokens;

fn check_valid_f32(value: f32) -> Result<f32> {
    match value.classify() {
        std::num::FpCategory::Normal => Ok(value),
        std::num::FpCategory::Zero => Ok(value),
        std::num::FpCategory::Infinite => Err(anyhow!("Invalid f32 value: {} Infinite", value)),
        std::num::FpCategory::Nan => Err(anyhow!("Invalid f32 value: {} NaN", value)),
        std::num::FpCategory::Subnormal => Err(anyhow!("Invalid f32 value: {} Subnormal", value)),
    }
}

fn into_model_reserves(
    reserves: Vec<(evm::PoolId, evm::VirtualReserves)>,
    dex_info: &evm::DexInfo,
) -> Result<Vec<solarium::PoolReserves<PoolId, TokenId>>> {
    reserves
        .into_iter()
        .map(|(id, r)| {
            let (token0, token1) = dex_info
                .lookup_pool_tokens(id)
                .ok_or_else(|| anyhow!("Pool id not found in dex info {:?}", id))?;
            anyhow::Ok(solarium::PoolReserves {
                token0: TokenId::Evm(token0),
                token1: TokenId::Evm(token1),
                pool_id: PoolId::Evm(id),
                value: solarium::VirtualReserveValues {
                    token_0: r
                        .token0
                        .to_f32()
                        .ok_or_else(|| anyhow!("Failed to convert to f32"))
                        .and_then(check_valid_f32)
                        .map_err(|e| anyhow!("{}: token0 reserve", e))?,
                    token_1: r
                        .token1
                        .to_f32()
                        .ok_or_else(|| anyhow!("Failed to convert token1 reserve to f32"))
                        .and_then(check_valid_f32)
                        .map_err(|e| anyhow!("{}, token1 reserve", e))?,
                    fee_multiplier: r
                        .fee_multiplier
                        .to_f32()
                        .ok_or_else(|| anyhow!("Failed to convert fee multiplier to f32"))
                        .and_then(check_valid_f32)
                        .map_err(|e| anyhow!("{}, fee multiplier", e))?,
                    max_swap_0: r
                        .max_swap0
                        .to_f32()
                        .ok_or_else(|| anyhow!("Failed to convert max swap 0 to f32"))
                        .and_then(check_valid_f32)
                        .map_err(|e| anyhow!("{}, max swap 0", e))?,
                    max_swap_1: r
                        .max_swap1
                        .to_f32()
                        .ok_or_else(|| anyhow!("Failed to convert max swap 1 to f32"))
                        .and_then(check_valid_f32)
                        .map_err(|e| anyhow!("{}, max swap 1", e))?,
                },
            })
        })
        .collect::<Result<Vec<_>>>()
}

fn into_model_bridges(bridges: Vec<Bridge>) -> HashSet<(TokenId, TokenId)> {
    bridges
        .into_iter()
        .fold(HashSet::new(), |mut set, Bridge::Evm(source, target)| {
            set.insert((TokenId::Evm(source.local_token()), target));
            set
        })
}

pub fn format_swaps_graph<T: std::fmt::Debug, Ty: petgraph::EdgeType>(
    graph: &Graph<TokenId, T, Ty>,
    token_label_fn: impl Fn(&TokenId) -> String,
    pool_label_fn: impl Fn(&T) -> String,
) -> Result<String> {
    let edge_attr =
        |_: &Graph<TokenId, T, Ty>, e: petgraph::graph::EdgeReference<'_, T>| -> String {
            let pool_id: &T = e.weight();

            format!("label = \"{}\" ", pool_label_fn(pool_id))
        };

    let node_attr = |_: &Graph<TokenId, T, Ty>,
                     (_, token_address): (petgraph::graph::NodeIndex, &TokenId)|
     -> String { format!("label = \"{}\" ", token_label_fn(token_address)) };

    // let graph_clone = self.graph.clone();
    let dot = Dot::with_attr_getters(
        graph,
        &[dot::Config::NodeNoLabel, dot::Config::EdgeNoLabel],
        &edge_attr,
        &node_attr,
    );

    let dot_string = format!("{:?}", dot);
    // let mut file = File::create(file_path)?;
    // file.write_all(dot_string.as_bytes())?;

    Ok(dot_string)
}

fn show_token_id(token_id: &TokenId, dex_info: &evm::DexInfo) -> String {
    match token_id {
        TokenId::Evm(token_address) => dex_info
            .tokens
            .get(token_address)
            .map(|token_info| {
                format!(
                    "{} ({})",
                    token_info.symbol,
                    token_address.blockchain().name()
                )
            })
            .unwrap_or_else(|| format!("{:?}", token_address)),
        TokenId::Solana(address) => format!("Solana Token ({:?})", address),
    }
}

fn show_pool_id(pool_id: &PoolId, dex_info: &evm::DexInfo) -> String {
    match pool_id {
        PoolId::Evm(evm::PoolId::UniswapV2(_)) => "U2".to_string(),
        PoolId::Evm(evm::PoolId::UniswapV3(address)) => dex_info
            .uniswap_v3_pools
            .get(address)
            .map(|pool_info| format!("U3 ({})", pool_info.fee as i32))
            .unwrap_or("U3".to_string()),
        PoolId::Evm(evm::PoolId::UniswapV4(id)) => dex_info
            .uniswap_v4_pools
            .get(id)
            .map(|pool_info| format!("U4 ({})", pool_info.fee.0))
            .unwrap_or("U4".to_string()),
        PoolId::Evm(evm::PoolId::PancakeSwap(address)) => dex_info
            .pancakeswap_pools
            .get(address)
            .map(|pool_info| format!("PancakeSwap ({})", pool_info.fee as i32))
            .unwrap_or("PancakeSwap".to_string()),
        PoolId::Solana(address) => format!("SOL ({:?})", address),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let graphs_path = Path::new("graphs");
    if !graphs_path.exists() {
        fs::create_dir_all(graphs_path)?;
    }

    {
        let mut file = File::create(format!("{}/test", graphs_path.display()))?;
        file.write_all(b"test")?;
    }

    let (dex_info, bridges, tokens_graph) = collect_pools().await?;

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
        evm::StateManager::init(&evm_pool_ids, dex_info.clone()).await?;

    println!("Initial reserves size: {}", initial_reserves.len());

    let model_reserves = into_model_reserves(initial_reserves, dex_info.as_ref())?;
    let model_bridges = into_model_bridges(bridges);

    let mut model = solarium::Model::<solarium::WgpuBackend, PoolId, TokenId, 1>::init(
        TokenId::Evm(evm::tokens::ethereum::USDC.address),
        model_reserves,
        &model_bridges,
    )?;

    println!("Initialized Model ({:?}) ", model.shape());

    let input_amount = 1000.0;

    model = model.optimize(input_amount, 500);

    println!("model output: {}", model.evaluate(input_amount));

    let mut count = 30;

    while count > 0 {
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

        model = model.update(into_model_reserves(reserves, dex_info.as_ref())?);
        model = model.optimize(input_amount, 20);
        let output = model.evaluate(input_amount);
        println!("model output: {}", output);
        if output > 1010.0 {
            match model
                .swaps_graph(|val| Some(*val), 0.01)
                .map_err(|err| anyhow!("Failed to generate swaps graph: {}", err))
                .and_then(|graph| {
                    format_swaps_graph(
                        &graph,
                        |token_id| show_token_id(token_id, dex_info.as_ref()),
                        |(weight, pool_id)| {
                            format!(
                                "{} {}",
                                weight,
                                pool_id.map_or("".to_string(), |id| show_pool_id(
                                    &id,
                                    dex_info.as_ref()
                                ))
                            )
                        },
                    )
                    .map_err(|err| anyhow!("Failed to format swaps graph: {}", err))
                })
                .and_then(|dot_string| {
                    let time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|err| anyhow!("Failed to get time: {}", err))?;
                    let mut file = File::create(format!(
                        "{}/{}.dot",
                        graphs_path.display(),
                        time.as_millis()
                    ))
                    .map_err(|err| anyhow!("Failed to create file: {}", err))?;
                    file.write_all(dot_string.as_bytes())
                        .map_err(|err| anyhow!("Failed to write to file: {}", err))
                }) {
                Ok(()) => {
                    println!("Swaps graph saved to file");
                }
                Err(err) => {
                    println!("{}", err);
                }
            }
        }
        // tokio::time::sleep(Duration::from_secs(5)).await;
        count -= 1;
    }

    Ok(())
}
