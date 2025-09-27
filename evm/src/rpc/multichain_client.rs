use crate::{
    Blockchain,
    blockchain::BlockNumber,
    evm_network,
    rpc::{
        self,
        client::{NetworkProvider, RpcClient},
    },
};
use alloy::providers::Provider;
use anyhow::Result;

pub struct MultichainClient {
    ethereum_client: RpcClient<evm_network::Ethereum, NetworkProvider<evm_network::Ethereum>>,
    bsc_client: RpcClient<evm_network::BSC, NetworkProvider<evm_network::BSC>>,
    arbitrum_client: RpcClient<evm_network::Arbitrum, NetworkProvider<evm_network::Arbitrum>>,
}

impl MultichainClient {
    pub async fn init_client() -> Result<Self> {
        let ethereum_client = rpc::client::init_client::<evm_network::Ethereum>().await?;
        let bsc_client = rpc::client::init_client::<evm_network::BSC>().await?;
        let arbitrum_client = rpc::client::init_client::<evm_network::Arbitrum>().await?;

        Ok(Self {
            ethereum_client,
            bsc_client,
            arbitrum_client,
        })
    }

    pub async fn get_block_number(&self, blockchain: Blockchain) -> Result<u64> {
        match blockchain {
            Blockchain::Ethereum => Ok(self.ethereum_client.get_block_number().await?.value()),
            Blockchain::BSC => Ok(self.bsc_client.get_block_number().await?.value()),
            Blockchain::Arbitrum => Ok(self.arbitrum_client.get_block_number().await?.value()),
        }
    }
}
