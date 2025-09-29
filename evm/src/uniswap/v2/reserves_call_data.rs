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
    uniswap::v2::PoolInfo,
    uniswap_internal::v2::{contract, pool::PoolAddress},
    utils::try_into_decimal,
    virtual_reserves::VirtualReserves,
};

#[derive(Debug, Clone)]
pub struct ReservesCallData<B: BlockchainNetwork> {
    pool_address: Address,
    token0_decimals: u32,
    token1_decimals: u32,
    _blockchain_marker: PhantomData<fn() -> B>,
}

impl<B: BlockchainNetwork> ReservesCallData<B> {
    pub fn create(pool_address: &PoolAddress, pool_info: &PoolInfo) -> Result<Self> {
        let PoolAddress(address, pool_blockchain) = pool_address;

        B::BLOCKCHAIN
            .same_as(*pool_blockchain)
            .and_then(|bc| bc.same_as(pool_info.token0.address.blockchain()))
            .and_then(|bc| bc.same_as(pool_info.token1.address.blockchain()))
            .map(|_| Self {
                pool_address: *address,
                token0_decimals: pool_info.token0.decimals,
                token1_decimals: pool_info.token1.decimals,
                _blockchain_marker: PhantomData,
            })
    }
}

impl<B: BlockchainNetwork> MulticallData<B> for ReservesCallData<B> {
    type Calls = [multicall::Multicall3::Call; 1];
    // type Output = VirtualReserves<PoolAddress>;
    type Output = (PoolAddress, VirtualReserves);

    fn size(&self) -> usize {
        1
    }

    fn to_calls(&self) -> Self::Calls {
        let reserves_call = contract::Pair::getReservesCall {};
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
            let reserves_output =
                contract::Pair::getReservesCall::abi_decode_returns(reserves_bytes)?;

            let reserve_0 = try_into_decimal(reserves_output.reserve0, self.token0_decimals)?;
            let reserve_1 = try_into_decimal(reserves_output.reserve1, self.token1_decimals)?;

            Ok((
                PoolAddress(self.pool_address, B::BLOCKCHAIN),
                Reserves {
                    token0: reserve_0,
                    token1: reserve_1,
                }
                .as_virtual_reserves(),
            ))
        }
    }
}
