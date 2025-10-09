use anyhow::Result;
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenAddress(pub(crate) Pubkey);

impl TokenAddress {
    pub fn decode_from_bytes(bytes: bytes::Bytes) -> Result<Self> {
        let mut b: &[u8] = bytes.as_ref();
        let pubkey = Pubkey::deserialize(&mut b)?;
        Ok(TokenAddress(pubkey))
    }
}

pub struct TokenInfo {
    pub decimals: u8,
    pub symbol: Option<String>,
}

pub struct Token {
    pub address: TokenAddress,
    pub info: TokenInfo,
}
