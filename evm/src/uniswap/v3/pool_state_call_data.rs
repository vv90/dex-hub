use std::marker::PhantomData;

use crate::{blockchain::BlockchainNetwork, rpc::call_data::CallData, uniswap_internal::v3::contract};
use alloy::{
    primitives::{Address, Bytes},
    sol_types::SolCall,
};
use anyhow::Result;

pub struct PoolStateCallData<B: BlockchainNetwork> {
    pub pool_address: Address,
    blockchain_network_marker: PhantomData<B>,
}

impl<B: BlockchainNetwork> PoolStateCallData<B> {
    pub fn new(pool_address: Address) -> Self {
        Self {
            pool_address,
            blockchain_network_marker: PhantomData,
        }
    }
}

impl<B: BlockchainNetwork> CallData<B> for PoolStateCallData<B> {
    type Output = contract::slot0Return;

    fn contract_address(&self) -> Address {
        self.pool_address
    }

    fn into_call_data(&self) -> Bytes {
        let call = contract::slot0Call {};

        call.abi_encode().into()
    }

    fn decode_call_output(&self, response: Bytes) -> Result<Self::Output> {
        let output = contract::slot0Call::abi_decode_returns(&response)?;
        Ok(output)
    }
}
