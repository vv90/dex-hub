use std::{collections::HashSet, sync::LazyLock};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenId {
    Evm(evm::tokens::TokenAddress),
    Solana(solana::tokens::TokenAddress),
}

pub enum Token {
    Evm(evm::tokens::Token),
    Solana(solana::tokens::Token),
}

pub const BLACKLIST: LazyLock<HashSet<TokenId>> =
    LazyLock::new(|| HashSet::from([TokenId::Evm(evm::tokens::ethereum::USD_OLD.address)]));
