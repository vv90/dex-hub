use rust_decimal::Decimal;

pub struct VirtualReserves<T: Copy + Clone> {
    pub pool_id: T,
    pub token0: Decimal,
    pub token1: Decimal,
    pub max_swap0: Decimal,
    pub max_swap1: Decimal,
}
