use alloy::{
    primitives::{Address, Bytes},
    sol_types::{SolCall, SolValue},
};
use anyhow::{Result, anyhow};

use crate::{
    blockchain::{Blockchain, BlockchainNetwork},
    chainlink::{
        chain_selector::{ChainSelector, chain_selector},
        contract,
        pool::PoolAddress,
    },
    multicall,
    rpc::multicall_data::MulticallData,
    tokens::TokenAddress,
};

pub struct RemoteTokensMulticallData<B: BlockchainNetwork> {
    pub pool_address: PoolAddress<B>,
    pub remote_blockchain: Blockchain,
}

impl<B: BlockchainNetwork> MulticallData<B> for RemoteTokensMulticallData<B> {
    const SIZE: usize = 1;
    type Calls = [multicall::Multicall3::Call; 1];
    type Output = Option<TokenAddress>;

    fn to_calls(&self) -> Self::Calls {
        let ChainSelector(chain_selector) = chain_selector(self.remote_blockchain);
        let remote_token_call = contract::TokenPool::getRemoteTokenCall {
            remoteChainSelector: chain_selector,
        };

        [multicall::Multicall3::Call {
            target: self.pool_address.0,
            callData: remote_token_call.abi_encode().into(),
        }]
    }

    fn decode_output(&self, response: &[Bytes]) -> Result<Self::Output> {
        let data = response
            .get(0)
            .ok_or(anyhow!("Missing remote token data"))?;
        if let Some(_) = response.get(1) {
            Err(anyhow!("Invalid response data size"))
        } else {
            let output = contract::TokenPool::getRemoteTokenCall::abi_decode_returns(data)
                .map_err(|e| anyhow!("Failed to decode remote token data: {}", e))?;
            if output.len() == 0 {
                Ok(None)
            } else {
                let address = Address::abi_decode(&output)
                    .map_err(|e| anyhow!("Failed to decode remote token address: {}", e))?;
                Ok(Some(TokenAddress(address, self.remote_blockchain)))
            }
        }
    }
}
