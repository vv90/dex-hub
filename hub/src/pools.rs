use crate::{
    graph::{TokenAdjacency, TokensGraph},
    tokens::TokenId,
};
use anyhow::Result;
use evm::Blockchain;
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

pub struct Bridge {
    from: TokenId,
    to: TokenId,
}

pub enum Pool {
    UniswapV2(evm::uniswap::v2::Pool),
    UniswapV3(evm::uniswap::v3::Pool),
    UniswapV4(evm::uniswap::v4::Pool),
    PancakeSwapV3(evm::pancakeswap::v3::Pool),
    Orca(solana::orca::Pool),
}

impl TokenAdjacency<PoolId> for evm::uniswap::v2::Pool {
    fn adjacent_tokens(&self) -> [TokenId; 2] {
        [
            TokenId::Evm(self.token0.address),
            TokenId::Evm(self.token1.address),
        ]
    }

    fn pool_id(&self) -> PoolId {
        PoolId::UniswapV2(self.address)
    }
}

impl TokenAdjacency<PoolId> for evm::uniswap::v3::Pool {
    fn adjacent_tokens(&self) -> [TokenId; 2] {
        [
            TokenId::Evm(self.token0.address),
            TokenId::Evm(self.token1.address),
        ]
    }

    fn pool_id(&self) -> PoolId {
        PoolId::UniswapV3(self.address)
    }
}

impl TokenAdjacency<PoolId> for evm::uniswap::v4::Pool {
    fn adjacent_tokens(&self) -> [TokenId; 2] {
        [
            TokenId::Evm(self.token0.address),
            TokenId::Evm(self.token1.address),
        ]
    }

    fn pool_id(&self) -> PoolId {
        PoolId::UniswapV4(self.pool_id)
    }
}

impl TokenAdjacency<PoolId> for evm::pancakeswap::v3::Pool {
    fn adjacent_tokens(&self) -> [TokenId; 2] {
        [
            TokenId::Evm(self.token0.address),
            TokenId::Evm(self.token1.address),
        ]
    }

    fn pool_id(&self) -> PoolId {
        PoolId::PancakeSwapV3(self.address)
    }
}

const MIN_VALUE: Decimal = dec!(1000.0);

const EVM_BLOCKCHAINS: [evm::Blockchain; 3] = [
    evm::Blockchain::Ethereum,
    evm::Blockchain::BSC,
    evm::Blockchain::Arbitrum,
];

async fn fill_pools_for_evm_blockchain(
    blockchain: Blockchain,
    graph: TokensGraph<PoolId>,
) -> Result<TokensGraph<PoolId>> {
    Ok(graph
        .with_pools(&evm::uniswap::v2::get_pools(blockchain, MIN_VALUE).await?)
        .with_pools(&evm::uniswap::v3::get_pools(blockchain, MIN_VALUE).await?)
        .with_pools(&evm::uniswap::v4::get_pools(blockchain, MIN_VALUE).await?)
        .with_pools(&evm::pancakeswap::v3::get_pools(blockchain, MIN_VALUE).await?))
}

pub async fn collect_pools() -> Result<()> {
    let mut tokens_graph: TokensGraph<PoolId> = TokensGraph::new();

    for blockchain in EVM_BLOCKCHAINS {
        tokens_graph = fill_pools_for_evm_blockchain(blockchain, tokens_graph).await?;
    }

    tokens_graph = tokens_graph.with_dead_end_tokens_removed();

    println!("Graph size: {}", tokens_graph.tokens_count());

    Ok(())
}
