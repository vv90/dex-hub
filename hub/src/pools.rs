use crate::{
    graph::{AdjacentTokens, DexGraph, TokenAdjacency},
    tokens::TokenId,
};
use anyhow::{Result, anyhow};
use evm::Blockchain;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoolId {
    Evm(evm::PoolId),
    Solana(solana::orca::PoolAddress),
}

// pub enum Pool {
//     UniswapV2(evm::uniswap::v2::Pool),
//     UniswapV3(evm::uniswap::v3::Pool),
//     UniswapV4(evm::uniswap::v4::Pool),
//     PancakeSwapV3(evm::pancakeswap::v3::Pool),
//     Orca(solana::orca::Pool),
// }

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
            TokenId::Evm(self.info.token0.address),
            TokenId::Evm(self.info.token1.address),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV2(self.address)))
    }
}

impl TokenAdjacency<TokensConnectionType> for evm::uniswap::v3::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.info.token0.address),
            TokenId::Evm(self.info.token1.address),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV3(self.address)))
    }
}

impl TokenAdjacency<TokensConnectionType> for evm::uniswap::v4::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.info.token0.address),
            TokenId::Evm(self.info.token1.address),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::Evm(evm::PoolId::UniswapV4(self.id)))
    }
}

impl TokenAdjacency<TokensConnectionType> for evm::pancakeswap::v3::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.info.token0.address),
            TokenId::Evm(self.info.token1.address),
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

async fn with_evm_blockchain_pools(
    blockchain: Blockchain,
    graph: DexGraph<TokensConnectionType>,
) -> Result<DexGraph<TokensConnectionType>> {
    Ok(graph
        .with_adjacent_tokens(&evm::uniswap::v2::get_pools(blockchain, MIN_VALUE).await?)
        .with_adjacent_tokens(&evm::uniswap::v3::get_pools(blockchain, MIN_VALUE).await?)
        .with_adjacent_tokens(&evm::uniswap::v4::get_pools(blockchain, MIN_VALUE).await?)
        .with_adjacent_tokens(&evm::pancakeswap::v3::get_pools(blockchain, MIN_VALUE).await?))
}

// async fn with_solana_blockchain_pools(
//     graph: DexGraph<TokensConnectionType>,
// ) -> Result<DexGraph<TokensConnectionType>> {
//     Ok(graph
//         .with_adjacent_tokens(&solana::orca::get_pools(MIN_VALUE.round().mantissa() as i32).await?))
// }

pub async fn collect_pools() -> Result<DexGraph<TokensConnectionType>> {
    let mut tokens_graph: DexGraph<TokensConnectionType> = DexGraph::new();

    for blockchain in EVM_BLOCKCHAINS {
        tokens_graph = with_evm_blockchain_pools(blockchain, tokens_graph).await?;
    }

    // tokens_graph = with_solana_blockchain_pools(tokens_graph).await?;

    let bridges =
        evm::chainlink::bridges::get_bridges(&EVM_BLOCKCHAINS, &REMOTE_CHAIN_SELECTORS).await?;
    let bridges = bridges
        .into_iter()
        .map(|bridge| bridge.try_into())
        .collect::<Result<Vec<Bridge>>>()?;

    tokens_graph = tokens_graph.with_adjacent_tokens(&bridges);
    // tokens_graph = tokens_graph.with_dead_end_tokens_removed();

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

    Ok(tokens_graph)
}
