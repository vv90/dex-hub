use alloy::{
    primitives::{Address, address},
    sol_types::SolValue,
};
use anyhow::{Result, anyhow};
use std::sync::LazyLock;

use crate::blockchain::Blockchain;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenAddress(pub(crate) Address, pub Blockchain);

impl TokenAddress {
    pub fn blockchain(&self) -> Blockchain {
        self.1
    }

    pub fn decode_from_bytes(bytes: bytes::Bytes, blockchain: Blockchain) -> Result<Self> {
        let address = Address::abi_decode(&bytes)
            .map_err(|e| anyhow!("Failed to decode token address: {}", e))?;

        Ok(TokenAddress(address, blockchain))
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub address: TokenAddress,
    pub decimals: u32,
    pub symbol: String,
}

pub mod ethereum {
    use super::*;

    pub const ETH: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("0x0000000000000000000000000000000000000000"),
            Blockchain::Ethereum,
        ),
        decimals: 18,
        symbol: "ETH".to_string(),
    });
    pub const USDT: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("dac17f958d2ee523a2206206994597c13d831ec7"),
            Blockchain::Ethereum,
        ),
        decimals: 6,
        symbol: "USDT".to_string(),
    });
    pub const USDC: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
            Blockchain::Ethereum,
        ),
        decimals: 6,
        symbol: "USDC".to_string(),
    });
    pub const WBTC: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"),
            Blockchain::Ethereum,
        ),
        decimals: 8,
        symbol: "WBTC".to_string(),
    });
    pub const WETH: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
            Blockchain::Ethereum,
        ),
        decimals: 18,
        symbol: "WETH".to_string(),
    });
    pub const SCLM: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("82081822932cf22e39d2fbec8047feb6117cd2f6"),
            Blockchain::Ethereum,
        ),
        decimals: 18,
        symbol: "SCLM".to_string(),
    });
    pub const AMPL: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("d46ba6d942050d489dbd938a2c909a5d5039a161"),
            Blockchain::Ethereum,
        ),
        decimals: 9,
        symbol: "AMPL".to_string(),
    });
    pub const UNI: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("1f9840a85d5af5bf1d1762f925bdaddc4201f984"),
            Blockchain::Ethereum,
        ),
        decimals: 18,
        symbol: "UNI".to_string(),
    });
    pub const WXRP: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("39fbbabf11738317a448031930706cd3e612e1b9"),
            Blockchain::Ethereum,
        ),
        decimals: 18,
        symbol: "WXRP".to_string(),
    });
    pub const SOL: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("7d27c2f1dac1615131a5e98be68ef1818fdfa53c"),
            Blockchain::Ethereum,
        ),
        decimals: 18,
        symbol: "SOL".to_string(),
    });
    pub const WDOGE: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("35a532d376ffd9a705d0bb319532837337a398e7"),
            Blockchain::Ethereum,
        ),
        decimals: 18,
        symbol: "WDOGE".to_string(),
    });
    pub const AAVE: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("7fc66500c84a76ad7e9c93437bfc5ac33e2ddae9"),
            Blockchain::Ethereum,
        ),
        decimals: 18,
        symbol: "AAVE".to_string(),
    });
    pub const LINK: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("514910771af9ca656af840dff83e8264ecf986ca"),
            Blockchain::Ethereum,
        ),
        decimals: 18,
        symbol: "LINK".to_string(),
    });

    pub const USD_OLD: LazyLock<Token> = LazyLock::new(|| Token {
        address: TokenAddress(
            address!("0xd233D1f6FD11640081aBB8db125f722b5dc729dc"),
            Blockchain::Ethereum,
        ),
        decimals: 9,
        symbol: "USD".to_string(),
    });
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use alloy::primitives::Address;
    use proptest::prelude::*;

    impl Arbitrary for TokenAddress {
        type Parameters = ();

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            any::<[u8; 20]>()
                .prop_map(|bytes| TokenAddress(Address::new(bytes), Blockchain::Ethereum))
                .boxed()
        }

        type Strategy = BoxedStrategy<Self>;
    }
}
