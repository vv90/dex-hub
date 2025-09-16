use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenAddress(pub(crate) Pubkey);

pub struct Token {
    pub address: TokenAddress,
    pub decimals: u8,
    pub symbol: Option<String>,
}
