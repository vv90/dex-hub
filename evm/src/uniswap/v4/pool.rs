use alloy::primitives::FixedBytes;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;

use crate::{
    blockchain::Blockchain, tokens::TokenAddress, uniswap_internal::utils::fee_amount_from_int,
};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fee(pub u32);

impl Fee {
    pub fn fee_amount(self) -> Decimal {
        fee_amount_from_int(self.0)
    }

    pub fn fee_multiplier(self) -> Decimal {
        dec!(1.0) - fee_amount_from_int(self.0)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoolId(pub FixedBytes<32>, pub Blockchain);

impl PoolId {
    pub fn blockchain(&self) -> Blockchain {
        self.1
    }
}

pub struct PoolInfo {
    pub fee: Fee,
    pub tick_spacing: u32,
    pub token0: TokenAddress,
    pub token1: TokenAddress,
}

pub struct Pool {
    pub id: PoolId,
    pub info: PoolInfo,
}
