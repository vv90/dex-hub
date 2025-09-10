use crate::whirlpool::Whirlpool;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

const Q64_SCALE: f64 = 18446744073709551616.0; // 2^64
const Q64_SCALE_DEC: Decimal = dec!(18446744073709551616);

// assuming constant product with concentrated liquidity
// token0 * token1 = k // within current tick (price = 1.0001^tick)
// max_swap amounts indicate tick boundaries
// dx = f(token0, token1, dy)
// dx = f(token0, token1, min(dy, max_swap_y))
#[derive(Debug, Clone, Copy)]
pub struct Reserves {
    pub token0: f32,
    pub token1: f32,
    pub max_swap0: f32,
    pub max_swap1: f32,
    pub fee_multiplier: f32, // 1.0 - fee
}

impl From<&Whirlpool> for Reserves {
    fn from(wp: &Whirlpool) -> Self {
        let sqrt_price_x64 = wp.sqrt_price;
        let liquidity = wp.liquidity;
        let tick = wp.tick_current_index;
        let tick_spacing = wp.tick_spacing;
        let fee = wp.fee_rate;

        Reserves {
            token0: reserve_x(sqrt_price_x64, liquidity) as f32,
            token1: reserve_y(sqrt_price_x64, liquidity) as f32,
            max_swap0: swap_limit_x(sqrt_price_x64, liquidity, tick, tick_spacing) as f32,
            max_swap1: swap_limit_y(sqrt_price_x64, liquidity, tick, tick_spacing) as f32,
            fee_multiplier: 1.0 - (fee as f32 / 1_000_000.0),
        }
    }
}

fn q64_to_decimal(q64: u128) -> Decimal {
    let integer_part = (q64 >> 64) as u64;
    let fractional_part = (q64 & 0xFFFFFFFFFFFFFFFF) as u64;

    let integer_decimal = Decimal::from(integer_part);
    let fractional_decimal = Decimal::from(fractional_part);

    integer_decimal + (fractional_decimal / Q64_SCALE_DEC)
}

fn floor_to_spacing(tick: i32, spacing: i32) -> i32 {
    // floor division to the nearest multiple of `spacing` below `tick`
    let r = tick % spacing; // remainder keeps the sign of tick in Rust
    if r >= 0 { tick - r } else { tick - r - spacing }
}

fn max_price_of_current_tick_interval(tick_current_index: i32, tick_spacing: u16) -> f64 {
    let s = tick_spacing as i32;
    let tick_lower = floor_to_spacing(tick_current_index, s);
    let tick_upper = tick_lower + s;
    // Price of B per A at a given tick is 1.0001^tick
    (1.0001_f64).powi(tick_upper)
}

fn tick_low(tick_index: i32, tick_spacing: u16) -> i32 {
    let s = tick_spacing as i32;
    tick_index.div_euclid(s) * s
}

fn tick_high(tick_index: i32, tick_spacing: u16) -> i32 {
    let low = tick_low(tick_index, tick_spacing);
    low + tick_spacing as i32
}

pub fn sqrt_price_x64_at_tick(tick: i32) -> u128 {
    let sqrt_price = 1.0001_f64.powf(tick as f64 / 2.0);
    let scaled = sqrt_price * Q64_SCALE;

    if !scaled.is_finite() || scaled <= 0.0 {
        0
    } else if scaled >= (u128::MAX as f64) {
        u128::MAX
    } else {
        scaled.floor() as u128
    }
}

fn reserve_x(sqrt_price_x64: u128, liquidity: u128) -> f64 {
    let scaled_price = sqrt_price_x64 as f64 / Q64_SCALE;
    let reserve = liquidity as f64 / scaled_price;

    reserve
}

fn reserve_y(sqrt_price_x64: u128, liquidity: u128) -> f64 {
    let scaled_price = sqrt_price_x64 as f64 / Q64_SCALE;
    let reserve = liquidity as f64 * scaled_price;

    reserve
}

fn swap_limit_x(
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current_index: i32,
    tick_spacing: u16,
) -> f64 {
    let low = tick_low(tick_current_index, tick_spacing);
    let sqrt_price_min = sqrt_price_x64_at_tick(low);
    let reserve_current = reserve_x(sqrt_price_x64, liquidity);
    let reserve_min = reserve_x(sqrt_price_min, liquidity);

    reserve_min - reserve_current
}

fn swap_limit_y(
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current_index: i32,
    tick_spacing: u16,
) -> f64 {
    let high = tick_high(tick_current_index, tick_spacing);
    let sqrt_price_max = sqrt_price_x64_at_tick(high);
    let reserve_current = reserve_y(sqrt_price_x64, liquidity);
    let reserve_max = reserve_y(sqrt_price_max, liquidity);

    reserve_max - reserve_current
}

#[cfg(test)]
mod tests {
    use solana_sdk::pubkey::Pubkey;

    use crate::whirlpool::WhirlpoolRewardInfo;

    use super::*;

    #[test]
    fn test_price_invariant() {
        let whirlpool = Whirlpool {
            discriminator: [63, 149, 209, 12, 225, 128, 99, 9],
            whirlpools_config: Pubkey::default(),
            whirlpool_bump: [255],
            tick_spacing: 4,
            fee_tier_index_seed: [4, 0],
            fee_rate: 400,
            protocol_fee_rate: 1300,
            liquidity: 1147357566074456,
            sqrt_price: 8438445474665081295,
            tick_current_index: -15643,
            protocol_fee_owed_a: 198891750,
            protocol_fee_owed_b: 45464122,
            token_mint_a: Pubkey::default(),
            token_vault_a: Pubkey::default(),
            fee_growth_global_a: 10356937066687460040,
            token_mint_b: Pubkey::default(),
            token_vault_b: Pubkey::default(),
            fee_growth_global_b: 1366275000703991779,
            reward_last_updated_timestamp: 1756892920,
            reward_infos: [
                WhirlpoolRewardInfo {
                    mint: Pubkey::default(),
                    vault: Pubkey::default(),
                    authority: Pubkey::default(),
                    emissions_per_second_x64: 0,
                    growth_global_x64: 4903944807059099939,
                },
                WhirlpoolRewardInfo {
                    mint: Pubkey::default(),
                    vault: Pubkey::default(),
                    authority: Pubkey::default(),
                    emissions_per_second_x64: 0,
                    growth_global_x64: 0,
                },
                WhirlpoolRewardInfo {
                    mint: Pubkey::default(),
                    vault: Pubkey::default(),
                    authority: Pubkey::default(),
                    emissions_per_second_x64: 0,
                    growth_global_x64: 0,
                },
            ],
        };

        let expected_price = q64_to_decimal(whirlpool.sqrt_price);
        let expected_price = expected_price * expected_price;

        let reserves = Reserves::from(&whirlpool);
        let price = Decimal::from_f64(reserves.token1 as f64 / reserves.token0 as f64).unwrap();

        let tolerance = dec!(0.0001);
        let abs_diff = (price - expected_price).abs();

        assert!(
            abs_diff <= tolerance,
            "Price difference exceeds tolerance. Expected: {}, Actual: {}",
            expected_price,
            price
        );
    }
}
