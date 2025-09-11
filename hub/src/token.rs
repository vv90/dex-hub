use std::{collections::HashSet, sync::LazyLock};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum PoolId {
    UniswapV2(evm::uniswap::v2::PoolAddress),
    UniswapV3(evm::uniswap::v3::PoolAddress),
    UniswapV4(evm::uniswap::v4::PoolId),
    PancakeSwapV3(evm::pancakeswap::v3::PoolAddress),
    Orca(SolanaBlockchain, [u8; 32]),
}

pub struct Bridge {
    from: TokenId,
    to: TokenId,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SolanaBlockchain {
    Solana,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenId {
    Evm(evm::tokens::TokenAddress),
    Solana(SolanaBlockchain, [u8; 32]),
}

pub const BLACKLIST: LazyLock<HashSet<TokenId>> =
    LazyLock::new(|| HashSet::from([TokenId::Evm(evm::tokens::ethereum::USD_OLD.address)]));
