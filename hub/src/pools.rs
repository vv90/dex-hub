use crate::{
    graph::{AdjacentTokens, DexGraph, TokenAdjacency},
    tokens::TokenId,
};
use anyhow::Result;
use evm::{
    Blockchain,
    chainlink::{self, bridges::Bridge},
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoolId {
    UniswapV2(evm::uniswap::v2::PoolAddress),
    UniswapV3(evm::uniswap::v3::PoolAddress),
    UniswapV4(evm::uniswap::v4::PoolId),
    PancakeSwapV3(evm::pancakeswap::v3::PoolAddress),
    Orca(solana::orca::PoolAddress),
}

pub enum Pool {
    UniswapV2(evm::uniswap::v2::Pool),
    UniswapV3(evm::uniswap::v3::Pool),
    UniswapV4(evm::uniswap::v4::Pool),
    PancakeSwapV3(evm::pancakeswap::v3::Pool),
    Orca(solana::orca::Pool),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokensConnectionType {
    Swap(PoolId),
    Bridge(chainlink::pool::PoolAddress),
}

impl TokenAdjacency<TokensConnectionType> for evm::uniswap::v2::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.token0.address),
            TokenId::Evm(self.token1.address),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::UniswapV2(self.address))
    }
}

impl TokenAdjacency<TokensConnectionType> for evm::uniswap::v3::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.token0.address),
            TokenId::Evm(self.token1.address),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::UniswapV3(self.address))
    }
}

impl TokenAdjacency<TokensConnectionType> for evm::uniswap::v4::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.token0.address),
            TokenId::Evm(self.token1.address),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::UniswapV4(self.pool_id))
    }
}

impl TokenAdjacency<TokensConnectionType> for evm::pancakeswap::v3::Pool {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Undirected(
            TokenId::Evm(self.token0.address),
            TokenId::Evm(self.token1.address),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Swap(PoolId::PancakeSwapV3(self.address))
    }
}

impl TokenAdjacency<TokensConnectionType> for chainlink::bridges::Bridge {
    fn adjacent_tokens(&self) -> AdjacentTokens {
        AdjacentTokens::Directed(
            TokenId::Evm(self.local_token()),
            TokenId::Evm(self.remote_token()),
        )
    }

    fn id(&self) -> TokensConnectionType {
        TokensConnectionType::Bridge(self.pool_address())
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
        TokensConnectionType::Swap(PoolId::Orca(self.address))
    }
}

const MIN_VALUE: Decimal = dec!(1000.0);

const EVM_BLOCKCHAINS: [evm::Blockchain; 3] = [
    evm::Blockchain::Ethereum,
    evm::Blockchain::BSC,
    evm::Blockchain::Arbitrum,
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

async fn with_solana_blockchain_pools(
    graph: DexGraph<TokensConnectionType>,
) -> Result<DexGraph<TokensConnectionType>> {
    Ok(graph
        .with_adjacent_tokens(&solana::orca::get_pools(MIN_VALUE.round().mantissa() as i32).await?))
}

pub async fn collect_pools() -> Result<()> {
    let mut tokens_graph: DexGraph<TokensConnectionType> = DexGraph::new();

    for blockchain in EVM_BLOCKCHAINS {
        tokens_graph = with_evm_blockchain_pools(blockchain, tokens_graph).await?;
    }

    tokens_graph = with_solana_blockchain_pools(tokens_graph).await?;
    // tokens_graph = tokens_graph
    // .with_adjacent_tokens(&evm::chainlink::bridges::get_bridges(&EVM_BLOCKCHAINS).await?);
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

    tokens_graph = tokens_graph.pruned();

    println!("Graph size after pruning: {}", tokens_graph.tokens_count());
    println!(
        "Graph components after pruning: {:?}",
        tokens_graph
            .components()
            .iter()
            .map(|component| component.len())
            .collect::<Vec<_>>()
    );

    Ok(())
}
