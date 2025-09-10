use alloy::network::Network;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::evm_network;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Blockchain {
    Ethereum,
    BSC,
    Arbitrum,
}

pub const ALL_BLOCKCHAINS: [Blockchain; 3] =
    [Blockchain::Ethereum, Blockchain::BSC, Blockchain::Arbitrum];

impl Blockchain {
    pub const fn name(&self) -> &'static str {
        match self {
            Blockchain::Ethereum => "ethereum",
            Blockchain::BSC => "bsc",
            Blockchain::Arbitrum => "arbitrum",
        }
    }

    pub fn same_as(self, other: Blockchain) -> Result<Self> {
        if self == other {
            Ok(self)
        } else {
            Err(anyhow::anyhow!("Blockchains mismatch"))
        }
    }
}

pub trait BlockchainNetwork: Network {
    const BLOCKCHAIN: Blockchain;
}

impl BlockchainNetwork for evm_network::Ethereum {
    const BLOCKCHAIN: Blockchain = Blockchain::Ethereum;
}

impl BlockchainNetwork for evm_network::BSC {
    const BLOCKCHAIN: Blockchain = Blockchain::BSC;
}

impl BlockchainNetwork for evm_network::Arbitrum {
    const BLOCKCHAIN: Blockchain = Blockchain::Arbitrum;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockNumber<B: BlockchainNetwork> {
    number: u64,
    _phantom: std::marker::PhantomData<B>,
}

impl<B: BlockchainNetwork> BlockNumber<B> {
    pub fn new(number: u64) -> Self {
        Self {
            number,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn value(&self) -> u64 {
        self.number
    }
}

impl<B: BlockchainNetwork> std::fmt::Display for BlockNumber<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.number)
    }
}
