use rust_decimal::Decimal;

use crate::virtual_reserves::VirtualReserves;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reserves {
    pub token0: Decimal,
    pub token1: Decimal,
    pub fee_multiplier: Decimal,
}

impl Reserves {
    pub fn as_virtual_reserves(&self) -> VirtualReserves {
        VirtualReserves {
            // pool_id: self.pool_id,
            token0: self.token0,
            token1: self.token1,
            max_swap0: self.token0,
            max_swap1: self.token1,
            fee_multiplier: self.fee_multiplier,
        }
    }
}
