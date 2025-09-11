use alloy::primitives::Address;
use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;

use crate::{blockchain::Blockchain, tokens::Token, uniswap_internal::utils::fee_amount_from_int};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoolAddress(pub(crate) Address, pub Blockchain);

/// The default factory enabled fee amounts, denominated in hundredths of bips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum Fee {
    Lowest = 100,
    Low = 500,
    Medium = 2500,
    High = 10000,
}

impl Default for Fee {
    fn default() -> Self {
        Fee::Medium
    }
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
            Fee::Medium => 50,
            Fee::High => 200,
        }
    }
}

pub fn fee_from_int(fee_amount: u32) -> Result<Fee> {
    match fee_amount {
        100 => Ok(Fee::Lowest),
        500 => Ok(Fee::Low),
        2500 => Ok(Fee::Medium),
        10000 => Ok(Fee::High),
        invalid_amount => Err(anyhow!("Invalid fee amount: {:?}", invalid_amount)),
    }
}

#[derive(Debug, Clone)]
pub struct Pool {
    pub address: PoolAddress,
    pub fee: Fee,
    pub token0: Token,
    pub token1: Token,
}
