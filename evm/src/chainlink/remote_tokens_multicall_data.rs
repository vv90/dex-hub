use std::marker::PhantomData;

use alloy::{primitives::Bytes, sol_types::SolCall};

use crate::{
    blockchain::BlockchainNetwork,
    chainlink::{contract, pool::PoolAddress},
    multicall,
    rpc::multicall_data::MulticallData,
};

#[derive(Debug)]
pub struct RemoteTokensDataDecodeError {
    pub message: String,
    pub pool_address: PoolAddress,
    pub remote_chain_selector: u64,
}

impl std::error::Error for RemoteTokensDataDecodeError {}

impl std::fmt::Display for RemoteTokensDataDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let PoolAddress(address, blockchain) = self.pool_address;
        write!(
            f,
            "Failed to decode remote tokens call data for {} chainlink pool {} and remote chain {}: {}",
            blockchain.name(),
            address,
            self.remote_chain_selector,
            self.message
        )
    }
}

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

    pub fn decode_error(&self, message: String) -> RemoteTokensDataDecodeError {
        RemoteTokensDataDecodeError {
            pool_address: self.pool_address,
            remote_chain_selector: self.remote_chain_selector,
            message,
        }
    }
}

impl<B: BlockchainNetwork> MulticallData<B> for RemoteTokensMulticallData<B> {
    type Calls = [multicall::Multicall3::Call; 1];
    type Output = Option<(u64, bytes::Bytes)>;
    type DecodeError = RemoteTokensDataDecodeError;

    fn size(&self) -> usize {
        1
    }

    fn to_calls(&self) -> Self::Calls {
        let remote_token_call = contract::TokenPool::getRemoteTokenCall {
            remoteChainSelector: self.remote_chain_selector,
        };

        [multicall::Multicall3::Call {
            target: self.pool_address.0,
            callData: remote_token_call.abi_encode().into(),
        }]
    }

    fn decode_output(&self, response: &[Bytes]) -> Result<Self::Output, Self::DecodeError> {
        let data = response
            .get(0)
            .ok_or_else(|| self.decode_error("Missing remote token data".to_string()))?;
        if let Some(_) = response.get(1) {
            Err(self.decode_error("Invalid response data size".to_string()))
        } else {
            let Bytes(output) = contract::TokenPool::getRemoteTokenCall::abi_decode_returns(data)
                .map_err(|e| {
                self.decode_error(format!("Failed to decode remote token data: {}", e))
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
