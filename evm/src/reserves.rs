use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reserves<T: Clone + Copy> {
    pub pool_id: T,
    pub token0: Decimal,
    pub token1: Decimal,
}
