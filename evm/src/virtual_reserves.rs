use rust_decimal::Decimal;

pub struct VirtualReserves {
    // pub pool_id: T,
    pub token0: Decimal,
    pub token1: Decimal,
    pub max_swap0: Decimal,
    pub max_swap1: Decimal,
    pub fee_multiplier: Decimal,
}
