use crate::blockchain::Blockchain;

const ETHEREUM_CHAIN_SELECTOR: u64 = 5009297550715157269;
const BSC_CHAIN_SELECTOR: u64 = 11344663589394136015;
const ARBITRUM_CHAIN_SELECTOR: u64 = 4949039107694359620;

pub struct ChainSelector(pub u64);

pub const fn chain_selector(blockchain: Blockchain) -> ChainSelector {
    match blockchain {
        Blockchain::Ethereum => ChainSelector(ETHEREUM_CHAIN_SELECTOR),
        Blockchain::BSC => ChainSelector(BSC_CHAIN_SELECTOR),
        Blockchain::Arbitrum => ChainSelector(ARBITRUM_CHAIN_SELECTOR),
    }
}
