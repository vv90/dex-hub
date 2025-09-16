use alloy::{
    primitives::{Address, Bytes},
    sol_types::SolCall,
};
use anyhow::Result;
use std::marker::PhantomData;

use crate::{
    blockchain::BlockchainNetwork, chainlink::contract, rpc::call_data::CallData,
    tokens::TokenAddress,
};

#[derive(Debug, Clone, Copy)]
pub struct GetConfiguredTokensCallData<B: BlockchainNetwork> {
    start_index: u64,
    max_count: u64,
    blockchain: PhantomData<B>,
}

impl<B: BlockchainNetwork> GetConfiguredTokensCallData<B> {
    pub fn new(start_index: u64, max_count: u64) -> Self {
        Self {
            start_index,
            max_count,
            blockchain: PhantomData,
        }
    }

    pub fn next(&self) -> Self {
        Self {
            start_index: self.start_index + self.max_count,
            max_count: self.max_count,
            blockchain: PhantomData,
        }
    }

    pub fn max_count(&self) -> u64 {
        self.max_count
    }
}

impl<B: BlockchainNetwork> CallData<B> for GetConfiguredTokensCallData<B> {
    type Output = Vec<TokenAddress>;

    fn contract_address(&self) -> Address {
        contract::token_admin_registry_address(B::BLOCKCHAIN)
    }

    fn into_call_data(&self) -> Bytes {
        contract::TokenAdminRegistry::getAllConfiguredTokensCall {
            startIndex: self.start_index,
            maxCount: self.max_count,
        }
        .abi_encode()
        .into()
    }

    fn decode_call_output(&self, data: Bytes) -> Result<Self::Output> {
        let output =
            contract::TokenAdminRegistry::getAllConfiguredTokensCall::abi_decode_returns(&data)?;

        Ok(output
            .into_iter()
            .map(|address| TokenAddress(address, B::BLOCKCHAIN))
            .collect())
    }
}
