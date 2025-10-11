use std::marker::PhantomData;

use alloy::{
    primitives::{Address, Bytes},
    sol_types::SolCall,
};
use rust_decimal_macros::dec;

use crate::{
    blockchain::BlockchainNetwork,
    multicall,
    reserves::Reserves,
    rpc::multicall_data::MulticallData,
    tokens::TokenInfo,
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

#[derive(Debug, Clone)]
pub struct ReservesCallDataDecodeError {
    pub message: String,
    pub pool_address: PoolAddress,
}

impl std::error::Error for ReservesCallDataDecodeError {}

impl std::fmt::Display for ReservesCallDataDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let PoolAddress(address, blockchain) = self.pool_address;
        write!(
            f,
            "Failed to decode reserves call data for {} Uniswap V2 pool {}: {}",
            blockchain.name(),
            address,
            self.message
        )
    }
}

impl<B: BlockchainNetwork> ReservesCallData<B> {
    pub fn new(pool_address: Address, token0: &TokenInfo, token1: &TokenInfo) -> Self {
        Self {
            pool_address,
            token0_decimals: token0.decimals,
            token1_decimals: token1.decimals,
            _blockchain_marker: PhantomData,
        }
    }

    fn decode_error(&self, message: String) -> ReservesCallDataDecodeError {
        ReservesCallDataDecodeError {
            message: message.into(),
            pool_address: PoolAddress(self.pool_address, B::BLOCKCHAIN),
        }
    }
}

impl<B: BlockchainNetwork> MulticallData<B> for ReservesCallData<B> {
    type Calls = [multicall::Multicall3::Call; 1];
    // type Output = VirtualReserves<PoolAddress>;
    type Output = (PoolAddress, VirtualReserves);
    type DecodeError = ReservesCallDataDecodeError;

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

    fn decode_output(&self, response: &[Bytes]) -> Result<Self::Output, Self::DecodeError> {
        let reserves_bytes = response
            .get(0)
            .ok_or_else(|| self.decode_error("Missing reserves data".into()))?;
        if let Some(_) = response.get(1) {
            Err(self.decode_error("Invalid response data size".into()))
        } else {
            let reserves_output = contract::Pair::getReservesCall::abi_decode_returns(
                reserves_bytes,
            )
            .map_err(|e| self.decode_error(format!("Failed to decode reserves data: {}", e)))?;

            let reserve_0 = try_into_decimal(reserves_output.reserve0, self.token0_decimals)
                .map_err(|e| {
                    self.decode_error(format!("Failed to convert reserve0 to decimal: {}", e))
                })?;
            let reserve_1 = try_into_decimal(reserves_output.reserve1, self.token1_decimals)
                .map_err(|e| {
                    self.decode_error(format!("Failed to convert reserve1 to decimal: {}", e))
                })?;

            Ok((
                PoolAddress(self.pool_address, B::BLOCKCHAIN),
                Reserves {
                    token0: reserve_0,
                    token1: reserve_1,
                    // uniswap v2 fee amount is fixed at 0.3%
                    fee_multiplier: dec!(0.997),
                }
                .as_virtual_reserves(),
            ))
        }
    }
}
