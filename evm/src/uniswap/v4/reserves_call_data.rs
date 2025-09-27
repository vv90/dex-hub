use std::marker::PhantomData;

use alloy::{
    primitives::{Bytes, FixedBytes},
    sol_types::SolCall,
};

use crate::{
    blockchain::BlockchainNetwork,
    multicall,
    rpc::multicall_data::MulticallData,
    tokens::{Token, TokenAddress},
    uniswap_internal::v4::{
        contract,
        pool::{Fee, PoolId},
    },
    utils::try_into_decimal,
    virtual_reserves::VirtualReserves,
};
use anyhow::{Result, anyhow};

pub struct ReservesCallData<B: BlockchainNetwork> {
    pool_id: FixedBytes<32>,
    token0_decimals: u32,
    token1_decimals: u32,
    fee: Fee,
    tick_spacing: u32,
    _blockchain_marker: PhantomData<B>,
}

impl<B: BlockchainNetwork> ReservesCallData<B> {
    pub fn create(
        pool_id: PoolId,
        token0: Token,
        token1: Token,
        fee: Fee,
        tick_spacing: u32,
    ) -> Result<Self> {
        let PoolId(pool_id, pool_blockchain) = pool_id;
        let TokenAddress(_token0_address, token0_blockchain) = token0.address;
        let TokenAddress(_token1_address, token1_blockchain) = token1.address;
        B::BLOCKCHAIN
            .same_as(pool_blockchain)
            .and_then(|bc| bc.same_as(token0_blockchain))
            .and_then(|bc| bc.same_as(token1_blockchain))
            .map(|_| Self {
                pool_id,
                token0_decimals: token0.decimals,
                token1_decimals: token1.decimals,
                fee,
                tick_spacing,
                _blockchain_marker: PhantomData,
            })
    }
}

impl<B: BlockchainNetwork> MulticallData<B> for ReservesCallData<B> {
    type Calls = [multicall::Multicall3::Call; 2];
    // type Output = VirtualReserves<PoolId>;
    type Output = VirtualReserves;

    fn size(&self) -> usize {
        2
    }

    fn to_calls(&self) -> Self::Calls {
        let slot0_call = contract::StateView::getSlot0Call {
            poolId: self.pool_id,
        };

        let slot0_call_data = slot0_call.abi_encode();

        let liquidity_call = contract::StateView::getLiquidityCall {
            poolId: self.pool_id,
        };

        let liquidity_call_data = liquidity_call.abi_encode();

        [
            multicall::Multicall3::Call {
                target: contract::state_view_address(B::BLOCKCHAIN),
                callData: slot0_call_data.into(),
            },
            multicall::Multicall3::Call {
                target: contract::state_view_address(B::BLOCKCHAIN),
                callData: liquidity_call_data.into(),
            },
        ]
    }

    fn decode_output(&self, response: &[Bytes]) -> Result<Self::Output> {
        let slot_0_bytes = response.get(0).ok_or(anyhow!("Missing slot0 data"))?;
        let liquidity_bytes = response.get(1).ok_or(anyhow!("Missing liquidity data"))?;

        if let Some(_) = response.get(2) {
            Err(anyhow!("Invalid response data size"))
        } else {
            let slot_0_output = contract::StateView::getSlot0Call::abi_decode_returns(slot_0_bytes)
                .map_err(|e| anyhow!("slot0call decode failed: {}", e))?;
            let liquidity_output =
                contract::StateView::getLiquidityCall::abi_decode_returns(liquidity_bytes)
                    .map_err(|e| anyhow!("liquidityCall decode failed: {}", e))?;

            let pool_state = crate::uniswap_internal::v3::pool_state::PoolState {
                sqrt_price_x96: slot_0_output.sqrtPriceX96,
                tick: slot_0_output.tick,
                liquidity: liquidity_output,
            };

            let reserve0 = pool_state.virtual_reserve_x();
            let reserve1 = pool_state.virtual_reserve_y();
            let max_swap0 = pool_state.swap_limit_x(self.tick_spacing as u16)?;
            let max_swap1 = pool_state.swap_limit_y(self.tick_spacing as u16)?;

            Ok(VirtualReserves {
                // pool_id: PoolId(self.pool_id, B::BLOCKCHAIN),
                token0: try_into_decimal(reserve0, self.token0_decimals)?,
                token1: try_into_decimal(reserve1, self.token1_decimals)?,
                max_swap0: try_into_decimal(max_swap0, self.token0_decimals)?,
                max_swap1: try_into_decimal(max_swap1, self.token1_decimals)?,
            })
        }
    }
}
