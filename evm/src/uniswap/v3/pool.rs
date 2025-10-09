use alloy::primitives::Address;
use anyhow::{Result, anyhow};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::{
    blockchain::Blockchain, tokens::TokenAddress, uniswap_internal::utils::fee_amount_from_int,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct PoolAddress(pub(crate) Address, pub Blockchain);

impl PoolAddress {
    pub fn blockchain(&self) -> Blockchain {
        self.1
    }
}

/// The default factory enabled fee amounts, denominated in hundredths of bips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Fee {
    Lowest = 100,
    Low = 500,
    Medium = 3000,
    High = 10000,
}

impl Fee {
    pub fn fee_amount(self) -> Decimal {
        fee_amount_from_int(self as u32)
    }

    pub fn fee_multiplier(self) -> Decimal {
        dec!(1.0) - fee_amount_from_int(self as u32)
    }

    pub fn tick_spacing(self) -> u16 {
        match self {
            Fee::Lowest => 1,
            Fee::Low => 10,
            Fee::Medium => 60,
            Fee::High => 200,
        }
    }
}

pub fn fee_from_int(fee_amount: u16) -> Result<Fee> {
    match fee_amount {
        100 => Ok(Fee::Lowest),
        500 => Ok(Fee::Low),
        3000 => Ok(Fee::Medium),
        10000 => Ok(Fee::High),
        invalid_amount => Err(anyhow!("Invalid fee amount: {:?}", invalid_amount)),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TickInfo {
    pub index: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub initialized: bool,
}

#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub fee: Fee,
    pub token0: TokenAddress,
    pub token1: TokenAddress,
}

#[derive(Debug, Clone)]
pub struct Pool {
    pub address: PoolAddress,
    pub info: PoolInfo,
}

// impl TokenAdjacency<dex::pool_id::PoolId> for Pool {
//     fn adjacent_tokens(&self) -> [TokenAddress; 2] {
//         [self.token0.address(), self.token1.address()]
//     }

//     fn pool_id(&self) -> dex::pool_id::PoolId {
//         let PoolAddress(pool_address, bc) = self.address;
//         dex::pool_id::PoolId::UniswapV3(pool_address, bc)
//     }
// }
