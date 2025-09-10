use rust_decimal::Decimal;

use crate::uniswap::v3::pool::PoolAddress;

pub struct Reserves {
    pub pool_address: PoolAddress,
    pub token0: Decimal,
    pub token1: Decimal,
    pub max_swap0: Decimal,
    pub max_swap1: Decimal,
}
