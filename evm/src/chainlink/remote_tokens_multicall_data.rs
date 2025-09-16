use std::marker::PhantomData;

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
    pool_address: PoolAddress,
    remote_blockchain: Blockchain,
    blockchain_marker: PhantomData<B>,
}

impl<B: BlockchainNetwork> RemoteTokensMulticallData<B> {
    pub fn new(pool_address: PoolAddress, remote_blockchain: Blockchain) -> Self {
        Self {
            pool_address,
            remote_blockchain,
            blockchain_marker: PhantomData,
        }
    }
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
                .map_err(|e| {
                    anyhow!(
                        "Failed to decode remote token data for {:?}, {:?}:\n{}",
                        self.pool_address,
                        self.remote_blockchain,
                        e
                    )
                })?;
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
