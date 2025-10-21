use std::collections::HashMap;

use crate::{
    graph::{AdjacentTokens, DexGraph, TokenAdjacency},
    tokens::TokenId,
};
use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoolId {
    Evm(evm::PoolId),
    Solana(solana::orca::PoolAddress),
}

pub enum Bridge {
    Evm(evm::chainlink::bridges::BridgeSource, TokenId),
    // Solana(SolanaBridge),
}

// impl From<solana::chainlink::chain_selector::ChainSelector> for ChainSelector {
//     fn from(value: solana::chainlink::chain_selector::ChainSelector) -> Self {
//         ChainSelector(value.0)
//     }
// }

// impl From<evm::chainlink::chain_selector::ChainSelector> for ChainSelector {
//     fn from(value: evm::chainlink::chain_selector::ChainSelector) -> Self {
//         ChainSelector(value.0)
//     }
// }

// impl Into<evm::chainlink::chain_selector::ChainSelector> for ChainSelector {
//     fn into(self) -> evm::chainlink::chain_selector::ChainSelector {
//         evm::chainlink::chain_selector::ChainSelector(self.0)
//     }
// }

// impl Into<solana::chainlink::chain_selector::ChainSelector> for ChainSelector {
//     fn into(self) -> solana::chainlink::chain_selector::ChainSelector {
//         solana::chainlink::chain_selector::ChainSelector(self.0)
//     }
// }

impl TryFrom<evm::chainlink::bridges::Bridge> for Bridge {
    type Error = anyhow::Error;

    fn try_from(value: evm::chainlink::bridges::Bridge) -> std::result::Result<Self, Self::Error> {
        let target = decode_bridge_target(value.target)?;
        Ok(Bridge::Evm(value.source, target))
    }
}

fn decode_bridge_target(bridge_target: evm::chainlink::bridges::BridgeTarget) -> Result<TokenId> {
    match bridge_target.chain_selector {
        evm::chainlink::chain_selector::ETHEREUM_CHAIN_SELECTOR => {
            evm::tokens::TokenAddress::decode_from_bytes(
                bridge_target.remote_token,
                evm::Blockchain::Ethereum,
            )
            .map(|token_address| TokenId::Evm(token_address))
        }
        evm::chainlink::chain_selector::BSC_CHAIN_SELECTOR => {
            evm::tokens::TokenAddress::decode_from_bytes(
                bridge_target.remote_token,
                evm::Blockchain::BSC,
            )
            .map(|token_address| TokenId::Evm(token_address))
        }
        evm::chainlink::chain_selector::ARBITRUM_CHAIN_SELECTOR => {
            evm::tokens::TokenAddress::decode_from_bytes(
                bridge_target.remote_token,
                evm::Blockchain::Arbitrum,
            )
            .map(|token_address| TokenId::Evm(token_address))
        }
        solana::chainlink::chain_selector::SOLANA_CHAIN_SELECTOR => {
            solana::tokens::TokenAddress::decode_from_bytes(bridge_target.remote_token)
                .map(|token_address| TokenId::Solana(token_address))
        }
        // TODO: SOLANA
        other => Err(anyhow!("unknown chain selector: {:?}", other)),
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokensConnectionType {
    Swap(PoolId),
    Bridge(evm::chainlink::pool::PoolAddress),
}

impl TokenAdjacency<TokensConnectionType> for evm::uniswap::v2::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.info.token0),
            TokenId::Evm(self.info.token1),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV2(self.address)))
    }
}

impl TokenAdjacency<TokensConnectionType> for evm::uniswap::v3::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.info.token0),
            TokenId::Evm(self.info.token1),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV3(self.address)))
    }
}

impl TokenAdjacency<TokensConnectionType> for evm::uniswap::v4::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.info.token0),
            TokenId::Evm(self.info.token1),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV4(self.id)))
    }
}

impl TokenAdjacency<TokensConnectionType> for evm::pancakeswap::v3::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.info.token0),
            TokenId::Evm(self.info.token1),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::PancakeSwap(self.address)))
    }
}

impl TokenAdjacency<TokensConnectionType> for Bridge {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        match self {
            Bridge::Evm(source, target) => {
                AdjacentTokens::Directed(TokenId::Evm(source.local_token()), *target)
            }
        }
    }

    fn id(&self) -> TokensConnectionType {
        match self {
            Bridge::Evm(source, _) => TokensConnectionType::Bridge(source.bridge_address()),
        }
    }
}

impl TokenAdjacency<TokensConnectionType> for solana::orca::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Solana(self.token0.address),
            TokenId::Solana(self.token1.address),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::Solana(self.address))
    }
}

const MIN_VALUE: Decimal = dec!(1000.0);

const EVM_BLOCKCHAINS: [evm::Blockchain; 3] = [
    evm::Blockchain::Ethereum,
    evm::Blockchain::BSC,
    evm::Blockchain::Arbitrum,
];

const REMOTE_CHAIN_SELECTORS: [u64; 3] = [
    evm::chainlink::chain_selector::ETHEREUM_CHAIN_SELECTOR,
    evm::chainlink::chain_selector::BSC_CHAIN_SELECTOR,
    evm::chainlink::chain_selector::ARBITRUM_CHAIN_SELECTOR,
    // solana::chainlink::chain_selector::SOLANA_CHAIN_SELECTOR,
];

// async fn with_evm_blockchain_pools(
//     blockchain: Blockchain,
//     graph: DexGraph<TokensConnectionType>,
// ) -> Result<(
//     DexGraph<TokensConnectionType>,
//     HashMap<evm::tokens::TokenAddress, evm::tokens::TokenInfo>,
// )> {
//     let mut tokens_map = HashMap::new();
//     let (pools_u2, tokens) = evm::uniswap::v2::get_pools(blockchain, MIN_VALUE).await?;
//     tokens_map.extend(tokens.into_iter());

//     let (pools_u3, tokens) = evm::uniswap::v3::get_pools(blockchain, MIN_VALUE).await?;
//     tokens_map.extend(tokens.into_iter());

//     let (pools_u4, tokens) = evm::uniswap::v4::get_pools(blockchain, MIN_VALUE).await?;
//     tokens_map.extend(tokens.into_iter());

//     let (pools_p3, tokens) = evm::pancakeswap::v3::get_pools(blockchain, MIN_VALUE).await?;
//     tokens_map.extend(tokens.into_iter());

//     Ok((
//         graph
//             .with_adjacent_tokens(&pools_u2)
//             .with_adjacent_tokens(&pools_u3)
//             .with_adjacent_tokens(&pools_u4)
//             .with_adjacent_tokens(&pools_p3),
//         tokens_map,
//     ))
// }

// async fn with_solana_blockchain_pools(
//     graph: DexGraph<TokensConnectionType>,
// ) -> Result<DexGraph<TokensConnectionType>> {
//     Ok(graph
//         .with_adjacent_tokens(&solana::orca::get_pools(MIN_VALUE.round().mantissa() as i32).await?))
// }

pub async fn collect_pools() -> Result<(evm::DexInfo, Vec<Bridge>, DexGraph<TokensConnectionType>)>
{
    let mut tokens_graph: DexGraph<TokensConnectionType> = DexGraph::new();
    let mut evm_tokens: HashMap<evm::tokens::TokenAddress, evm::tokens::TokenInfo> = HashMap::new();
    let mut evm_pools_u2: HashMap<evm::uniswap::v2::PoolAddress, evm::uniswap::v2::PoolInfo> =
        HashMap::new();
    let mut evm_pools_u3: HashMap<evm::uniswap::v3::PoolAddress, evm::uniswap::v3::PoolInfo> =
        HashMap::new();
    let mut evm_pools_u4: HashMap<evm::uniswap::v4::PoolId, evm::uniswap::v4::PoolInfo> =
        HashMap::new();
    let mut evm_pools_p3: HashMap<
        evm::pancakeswap::v3::PoolAddress,
        evm::pancakeswap::v3::PoolInfo,
    > = HashMap::new();

    for blockchain in EVM_BLOCKCHAINS {
        let (pools_u2, tokens) = evm::uniswap::v2::get_pools(blockchain, MIN_VALUE).await?;
        tokens_graph = tokens_graph.with_adjacent_tokens(&pools_u2);
        evm_tokens.extend(tokens.into_iter());
        evm_pools_u2.extend(
            pools_u2
                .into_iter()
                .map(|evm::uniswap::v2::Pool { address, info }| (address, info)),
        );

        let (pools_u3, tokens) = evm::uniswap::v3::get_pools(blockchain, MIN_VALUE).await?;
        tokens_graph = tokens_graph.with_adjacent_tokens(&pools_u3);
        evm_pools_u3.extend(
            pools_u3
                .into_iter()
                .map(|evm::uniswap::v3::Pool { address, info }| (address, info)),
        );
        evm_tokens.extend(tokens.into_iter());

        let (pools_u4, tokens) = evm::uniswap::v4::get_pools(blockchain, MIN_VALUE).await?;
        tokens_graph = tokens_graph.with_adjacent_tokens(&pools_u4);
        evm_tokens.extend(tokens.into_iter());
        evm_pools_u4.extend(
            pools_u4
                .into_iter()
                .map(|evm::uniswap::v4::Pool { id, info }| (id, info)),
        );

        let (pools_p3, tokens) = evm::pancakeswap::v3::get_pools(blockchain, MIN_VALUE).await?;
        tokens_graph = tokens_graph.with_adjacent_tokens(&pools_p3);
        evm_tokens.extend(tokens.into_iter());
        evm_pools_p3.extend(
            pools_p3
                .into_iter()
                .map(|evm::pancakeswap::v3::Pool { address, info }| (address, info)),
        );
    }

    // tokens_graph = with_solana_blockchain_pools(tokens_graph).await?;

    let bridges =
        evm::chainlink::bridges::get_bridges(&EVM_BLOCKCHAINS, &REMOTE_CHAIN_SELECTORS).await?;
    let bridges = bridges
        .into_iter()
        .map(|bridge| bridge.try_into())
        .collect::<Result<Vec<Bridge>>>()?;

    tokens_graph = tokens_graph.with_adjacent_tokens(&bridges);

    println!("Graph size: {}", tokens_graph.tokens_count());
    println!(
        "Graph components: {:?}",
        tokens_graph
            .components()
            .iter()
            .map(|component| component.len())
            .collect::<Vec<_>>()
    );

    tokens_graph = tokens_graph.pruned(TokenId::Evm(evm::tokens::ethereum::USDC.address));

    println!("Graph size after pruning: {}", tokens_graph.tokens_count());
    let (u2, u3, u4, p3, s) =
        tokens_graph
            .adjacency_ids()
            .fold(
                (0, 0, 0, 0, 0),
                |(u2, u3, u4, p3, s), adj_id| match adj_id {
                    TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV2(_))) => {
                        (u2 + 1, u3, u4, p3, s)
                    }
                    TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV3(_))) => {
                        (u2, u3 + 1, u4, p3, s)
                    }
                    TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV4(_))) => {
                        (u2, u3, u4 + 1, p3, s)
                    }
                    TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::PancakeSwap(_))) => {
                        (u2, u3, u4, p3 + 1, s)
                    }
                    TokensConnectionType::Swap(PoolId::Solana(_)) => (u2, u3, u4, p3, s + 1),
                    TokensConnectionType::Bridge(_) => (u2, u3, u4, p3, s),
                },
            );
    println!("Uniswap V2 pools after pruning: {}", u2);
    println!("Uniswap V3 pools after pruning: {}", u3);
    println!("Uniswap V4 pools after pruning: {}", u4);
    println!("PancakeSwap pools after pruning: {}", p3);
    println!("Solana pools after pruning: {}", s);

    let dex_info = evm::DexInfo {
        tokens: evm_tokens
            .into_iter()
            .filter(|(address, _)| tokens_graph.contains_token(TokenId::Evm(*address)))
            .collect(),
        uniswap_v2_pools: evm_pools_u2
            .into_iter()
            .filter(|(id, info)| {
                tokens_graph.contains_adjacency(
                    TokenId::Evm(info.token0),
                    TokenId::Evm(info.token1),
                    &TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV2(*id))),
                )
            })
            .collect(),
        uniswap_v3_pools: evm_pools_u3
            .into_iter()
            .filter(|(id, info)| {
                tokens_graph.contains_adjacency(
                    TokenId::Evm(info.token0),
                    TokenId::Evm(info.token1),
                    &TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV3(*id))),
                )
            })
            .collect(),
        uniswap_v4_pools: evm_pools_u4
            .into_iter()
            .filter(|(id, info)| {
                tokens_graph.contains_adjacency(
                    TokenId::Evm(info.token0),
                    TokenId::Evm(info.token1),
                    &TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV4(*id))),
                )
            })
            .collect(),
        pancakeswap_pools: evm_pools_p3
            .into_iter()
            .filter(|(id, info)| {
                tokens_graph.contains_adjacency(
                    TokenId::Evm(info.token0),
                    TokenId::Evm(info.token1),
                    &TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::PancakeSwap(*id))),
                )
            })
            .collect(),
    };

    println!("Tokens map size: {}", dex_info.tokens.len());
    println!("Uniswap V2 map size: {}", dex_info.uniswap_v2_pools.len());
    println!("Uniswap V3 map size: {}", dex_info.uniswap_v3_pools.len());
    println!("Uniswap V4 map size: {}", dex_info.uniswap_v4_pools.len());
    println!("PancakeSwap map size: {}", dex_info.pancakeswap_pools.len());

    Ok((dex_info, bridges, tokens_graph))
}
