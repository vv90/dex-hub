use std::marker::PhantomData;

use alloy::{primitives::Bytes, sol_types::SolCall};
use anyhow::{Result, anyhow};

use crate::{
    blockchain::BlockchainNetwork,
    chainlink::{contract, pool::PoolAddress},
    multicall,
    rpc::multicall_data::MulticallData,
};

pub struct RemoteTokensMulticallData<B: BlockchainNetwork> {
    pool_address: PoolAddress,
    remote_chain_selector: u64,
    blockchain_marker: PhantomData<B>,
}

impl<B: BlockchainNetwork> RemoteTokensMulticallData<B> {
    pub fn new(pool_address: PoolAddress, remote_chain_selector: u64) -> Self {
        Self {
            pool_address,
            remote_chain_selector,
            blockchain_marker: PhantomData,
        }
    }
}

impl<B: BlockchainNetwork> MulticallData<B> for RemoteTokensMulticallData<B> {
    
    type Calls = [multicall::Multicall3::Call; 1];
    type Output = Option<(u64, bytes::Bytes)>;

    fn size(&self) -> usize {1} 
    
    fn to_calls(&self) -> Self::Calls {
        let remote_token_call = contract::TokenPool::getRemoteTokenCall {
            remoteChainSelector: self.remote_chain_selector,
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
            let Bytes(output) = contract::TokenPool::getRemoteTokenCall::abi_decode_returns(data)
                .map_err(|e| {
                anyhow!(
                    "Failed to decode remote token data for {:?}, {:?}:\n{}",
                    self.pool_address,
                    self.remote_chain_selector,
                    e
                )
            })?;
            if output.len() == 0 {
                Ok(None)
            } else {
                // let address = Address::abi_decode(&output)
                //     .map_err(|e| anyhow!("Failed to decode remote token address: {}", e))?;
                Ok(Some((self.remote_chain_selector, output)))
            }
        }
    }
}
