use std::marker::PhantomData;

use alloy::{
    primitives::{Address, Bytes},
    sol_types::SolCall,
};
use anyhow::{Result, anyhow};

use crate::{
    blockchain::BlockchainNetwork,
    multicall,
    reserves::Reserves,
    rpc::multicall_data::MulticallData,
    tokens::{Token, TokenAddress},
    uniswap_internal::v2::{contract, pool::PoolAddress},
    utils::try_into_decimal,
};

#[derive(Debug, Clone)]
pub struct ReservesCallData<B: BlockchainNetwork> {
    pool_address: Address,
    token0_decimals: u32,
    token1_decimals: u32,
    _blockchain_marker: PhantomData<fn() -> B>,
}

impl<B: BlockchainNetwork> ReservesCallData<B> {
    pub fn create(pool_address: PoolAddress, token0: Token, token1: Token) -> Result<Self> {
        let PoolAddress(address, pool_blockchain) = pool_address;
        let TokenAddress(_token0_address, token0_blockchain) = token0.address;
        let TokenAddress(_token1_address, token1_blockchain) = token1.address;

        B::BLOCKCHAIN
            .same_as(pool_blockchain)
            .and_then(|bc| bc.same_as(token0_blockchain))
            .and_then(|bc| bc.same_as(token1_blockchain))
            .map(|_| Self {
                pool_address: address,
                token0_decimals: token0.decimals,
                token1_decimals: token1.decimals,
                _blockchain_marker: PhantomData,
            })
    }
}

impl<B: BlockchainNetwork> MulticallData<B> for ReservesCallData<B> {
    const SIZE: usize = 1;
    type Calls = [multicall::Multicall3::Call; 1];
    type Output = Reserves<PoolAddress>;

    fn to_calls(&self) -> Self::Calls {
        let reserves_call = contract::getReservesCall {};
        let reserves_call_data = reserves_call.abi_encode();
        [multicall::Multicall3::Call {
            target: self.pool_address,
            callData: reserves_call_data.into(),
        }]
    }

    fn decode_output(&self, response: &[Bytes]) -> Result<Self::Output> {
        let reserves_bytes = response.get(0).ok_or(anyhow!("Missing reserves data"))?;
        if let Some(_) = response.get(1) {
            Err(anyhow!("Invalid response data size"))
        } else {
            let reserves_output = contract::getReservesCall::abi_decode_returns(reserves_bytes)?;

            let reserve_0 = try_into_decimal(reserves_output.reserve0, self.token0_decimals)?;
            let reserve_1 = try_into_decimal(reserves_output.reserve1, self.token1_decimals)?;

            Ok(Reserves {
                pool_id: PoolAddress(self.pool_address, B::BLOCKCHAIN),
                token0: reserve_0,
                token1: reserve_1,
            })
        }
    }
}
