use std::marker::PhantomData;

use crate::{
    blockchain::BlockchainNetwork,
    multicall,
    rpc::multicall_data::MulticallData,
    tokens::TokenInfo,
    uniswap_internal::v3::{
        contract,
        pool::{Fee, PoolAddress},
        pool_state::PoolState,
    },
    utils::try_into_decimal,
    virtual_reserves::VirtualReserves,
};
use alloy::{
    primitives::{Address, Bytes},
    sol_types::SolCall,
};

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
            "Failed to decode reserves call data for {} Uniswap V3 pool {}: {}",
            blockchain.name(),
            address,
            self.message
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReservesCallData<B: BlockchainNetwork> {
    pool_address: Address,
    token0_decimals: u32,
    token1_decimals: u32,
    fee: Fee,
    _blockchain_marker: PhantomData<B>,
}

impl<B: BlockchainNetwork> ReservesCallData<B> {
    pub fn new(pool_address: Address, token0: &TokenInfo, token1: &TokenInfo, fee: Fee) -> Self {
        Self {
            pool_address,
            token0_decimals: token0.decimals,
            token1_decimals: token1.decimals,
            fee,
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
    type Calls = [multicall::Multicall3::Call; 2];
    // type Output = VirtualReserves<PoolAddress>;
    type Output = (PoolAddress, VirtualReserves);
    type DecodeError = ReservesCallDataDecodeError;

    fn size(&self) -> usize {
        2
    }

    fn to_calls(&self) -> Self::Calls {
        let slot0_call = contract::Pool::slot0Call {};
        let slot0_call_data = slot0_call.abi_encode();

        let liquidity_call = contract::Pool::liquidityCall {};
        let liquidity_call_data = liquidity_call.abi_encode();

        [
            multicall::Multicall3::Call {
                target: self.pool_address,
                callData: slot0_call_data.into(),
            },
            multicall::Multicall3::Call {
                target: self.pool_address,
                callData: liquidity_call_data.into(),
            },
        ]
    }

    fn decode_output(&self, response: &[Bytes]) -> Result<Self::Output, Self::DecodeError> {
        let slot_0_bytes = response
            .get(0)
            .ok_or_else(|| self.decode_error("Missing slot0 data".into()))?;
        let liquidity_bytes = response
            .get(1)
            .ok_or_else(|| self.decode_error("Missing liquidity data".into()))?;

        if let Some(_) = response.get(2) {
            Err(self.decode_error("Invalid response data size".into()))
        } else {
            let slot_0_output = contract::Pool::slot0Call::abi_decode_returns(slot_0_bytes)
                .map_err(|e| self.decode_error(format!("slot0call decode failed: {}", e)))?;
            let liquidity_output = contract::Pool::liquidityCall::abi_decode_returns(
                liquidity_bytes,
            )
            .map_err(|e| self.decode_error(format!("liquidityCall decode failed: {}", e)))?;

            let pool_state = PoolState {
                sqrt_price_x96: slot_0_output.sqrtPriceX96,
                tick: slot_0_output.tick,
                liquidity: liquidity_output,
            };

            let reserve0 = pool_state.virtual_reserve_x();
            let reserve1 = pool_state.virtual_reserve_y();
            let max_swap0 = pool_state
                .swap_limit_x(self.fee.tick_spacing())
                .map_err(|e| {
                    self.decode_error(format!("swap_limit_x calculation failed: {}", e))
                })?;
            let max_swap1 = pool_state
                .swap_limit_y(self.fee.tick_spacing())
                .map_err(|e| {
                    self.decode_error(format!("swap_limit_y calculation failed: {}", e))
                })?;

            Ok((
                PoolAddress(self.pool_address, B::BLOCKCHAIN),
                VirtualReserves {
                    token0: try_into_decimal(reserve0, self.token0_decimals).map_err(|e| {
                        self.decode_error(format!("token0 reserve conversion failed: {}", e))
                    })?,
                    token1: try_into_decimal(reserve1, self.token1_decimals).map_err(|e| {
                        self.decode_error(format!("token1 reserve conversion failed: {}", e))
                    })?,
                    max_swap0: try_into_decimal(max_swap0, self.token0_decimals).map_err(|e| {
                        self.decode_error(format!("max_swap0 conversion failed: {}", e))
                    })?,
                    max_swap1: try_into_decimal(max_swap1, self.token1_decimals).map_err(|e| {
                        self.decode_error(format!("max_swap1 conversion failed: {}", e))
                    })?,
                    fee_multiplier: self.fee.fee_multiplier(),
                },
            ))
        }
    }
}
