use alloy::{
    primitives::{Address, Bytes},
    sol_types::SolCall,
};
use anyhow::{Result, anyhow};
use std::marker::PhantomData;

use crate::{
    blockchain::BlockchainNetwork,
    chainlink::{contract, pool::PoolAddress},
    rpc::call_data::CallData,
    tokens::TokenAddress,
};

pub struct GetPoolsCallData<'a, B: BlockchainNetwork> {
    token_addresses: &'a [TokenAddress],
    blockchain: PhantomData<B>,
}

impl<'a, B: BlockchainNetwork> GetPoolsCallData<'a, B> {
    pub fn create(token_addresses: &'a [TokenAddress]) -> Result<Self> {
        for token_address in token_addresses {
            let TokenAddress(_, blockchain) = token_address;

            if *blockchain == B::BLOCKCHAIN {
                continue;
            } else {
                return Err(anyhow!(
                    "Failed to create GetPoolsCallData<{}>. Invalid token blockchain: {}",
                    B::BLOCKCHAIN.name(),
                    blockchain.name()
                ));
            }
        }

        Ok(Self {
            token_addresses,
            blockchain: PhantomData,
        })
    }
}

impl<'a, B: BlockchainNetwork> CallData<B> for GetPoolsCallData<'a, B> {
    type Output = Vec<PoolAddress<B>>;

    fn contract_address(&self) -> Address {
        contract::token_admin_registry_address(B::BLOCKCHAIN)
    }

    fn into_call_data(&self) -> Bytes {
        contract::TokenAdminRegistry::getPoolsCall {
            tokens: self
                .token_addresses
                .iter()
                .map(|TokenAddress(address, _)| *address)
                .collect(),
        }
        .abi_encode()
        .into()
    }

    fn decode_call_output(&self, data: Bytes) -> Result<Self::Output> {
        let output = contract::TokenAdminRegistry::getPoolsCall::abi_decode_returns(&data)?;

        Ok(output
            .into_iter()
            // .zip(self.token_addresses.iter())
            .map(|pool_address| (PoolAddress::new(pool_address)))
            .collect())
    }
}
