use crate::blockchain::Blockchain;

pub const ETHEREUM_CHAIN_SELECTOR: u64 = 5009297550715157269;
pub const BSC_CHAIN_SELECTOR: u64 = 11344663589394136015;
pub const ARBITRUM_CHAIN_SELECTOR: u64 = 4949039107694359620;

pub const fn chain_selector(blockchain: Blockchain) -> u64 {
    match blockchain {
        Blockchain::Ethereum => ETHEREUM_CHAIN_SELECTOR,
        Blockchain::BSC => BSC_CHAIN_SELECTOR,
        Blockchain::Arbitrum => ARBITRUM_CHAIN_SELECTOR,
    }
}
