use crate::uniswap::{
    tick_math::{self, tick_high},
    utils::q_64_96_to_decimal,
};
use alloy::primitives::{
    U160, U256, U512,
    aliases::{I24, U24},
};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolState {
    pub sqrt_price_x96: U160,
    pub liquidity: u128,
    pub tick: I24,
}

fn virtual_reserve_x(sqrt_price_x96: U160, liquidity: u128) -> U256 {
    // For reserve_0: we need L * 2^96 / sqrtP (to account for the Q64.96 format)
    // This produces a number in Q0.0 format (regular integer)
    let liquidity_x96: U256 = U256::from(liquidity) << 96;

    // logically, zero sqrt_price_x96 is an undefined behavior and supposed to be represented as error
    // but in practice, the same case in virtual_reserve_y would return zero because it does not involve division by sqrt_price_x96
    // also, sqrt_price_x96 = 0 is no different from liquidity = 0 from the practical perspective
    // both mean empty reserves and the calling code should be able to handle empty reserves even if sqrt_price_x96=0 is represented as error
    // so zero is returned in case of sqrt_price_x96 = 0 to be consistent with virtual_reserve_y
    let reserve_x96 = liquidity_x96
        .checked_div(U256::from(sqrt_price_x96))
        .unwrap_or(U256::ZERO);
    reserve_x96
}

fn virtual_reserve_y(sqrt_price_x96: U160, liquidity: u128) -> U256 {
    let q_96 = U512::from(1u128 << 96);
    // For reserve_1: we need L * sqrtP / 2^96
    // This also produces a number in Q0.0 format
    let liquidity_x96: U512 = U512::from(liquidity);
    let reserve_x96 = U256::from(liquidity_x96 * U512::from(sqrt_price_x96) / q_96);
    reserve_x96
}

impl PoolState {
    pub fn virtual_reserve_x(&self) -> U256 {
        virtual_reserve_x(self.sqrt_price_x96, self.liquidity)
    }

    pub fn virtual_reserve_y(&self) -> U256 {
        virtual_reserve_y(self.sqrt_price_x96, self.liquidity)
    }

    pub fn swap_limit_x(&self, tick_spacing: u16) -> Result<U256> {
        let spacing = U24::from(tick_spacing);
        let tick_low = tick_math::tick_low(self.tick, spacing)?;
        let sqrt_price_min_x96 = tick_math::sqrt_price_at_tick(tick_low)
            .map_err(|e| anyhow!("sqrt_price_at_tick failed: {}\n{:?}", e, self))?;

        let reserve_current = virtual_reserve_x(self.sqrt_price_x96, self.liquidity);
        let reserve_min = virtual_reserve_x(sqrt_price_min_x96, self.liquidity);
        // let liquidity_x96: U256 = U256::from(self.liquidity) << 96;
        // let reserve_current = liquidity_x96 / U256::from(self.sqrt_price_x96);
        // let reserve_min = liquidity_x96 / U256::from(sqrt_price_min_x96);

        reserve_min.checked_sub(reserve_current).ok_or(
                anyhow!(
                    "Failed to calculate swap limit x (subtraction caused overflow): reserve_min: {}\n reserve_current: {}\n tick: {}\n tick_spacing: {}\n sqrt_price_current: {}\n sqrt_price_min: {}",
                    reserve_min,
                    reserve_current,
                    self.tick,
                    tick_spacing,
                    q_64_96_to_decimal(self.sqrt_price_x96),
                    q_64_96_to_decimal(sqrt_price_min_x96),
                )
        )
        // let swap_max = reserve_min - reserve_current;
        // swap_max.try_into().map_err(|e| {
        //     anyhow!(
        //         "Failed to convert swap_max to u128 {}\n reserve_min: {}\n reserve_current: {}\n tick: {}\n tick_spacing: {}\n sqrt_price_current: {}\n sqrt_price_min: {}\n{}",
        //         swap_max,
        //         reserve_min,
        //         reserve_current,
        //         self.tick,
        //         tick_spacing,
        //         q_64_96_to_decimal(self.sqrt_price_x96),
        //         q_64_96_to_decimal(sqrt_price_min_x96),
        //         e
        //     )
        // })
    }

    pub fn swap_limit_y(&self, tick_spacing: u16) -> Result<U256> {
        let spacing = U24::from(tick_spacing);
        let tick_high = tick_high(self.tick, spacing)?;
        let sqrt_price_max_x96 = tick_math::sqrt_price_at_tick(tick_high)
            .map_err(|e| anyhow!("sqrt_price_at_tick failed: {}\n{:?}", e, self))?;

        let reserve_current = virtual_reserve_y(self.sqrt_price_x96, self.liquidity);
        let reserve_max = virtual_reserve_y(sqrt_price_max_x96, self.liquidity);

        // let q_96 = U256::from(1u128 << 96);
        // // For reserve_1: we need L * sqrtP / 2^96
        // // This also produces a number in Q0.0 format
        // let liquidity_x96: U256 = U256::from(self.liquidity);
        // let reserve_current = liquidity_x96 * U256::from(self.sqrt_price_x96) / q_96;
        // let reserve_max = liquidity_x96 * U256::from(sqrt_price_max_x96) / q_96;

        // let swap_max = reserve_max - reserve_current;
        // swap_max.try_into().map_err(|e| {
        //     anyhow!(
        //         "Failed to convert swap_max to u128 {}\n reserve_current: {}\n reserve_max: {}\n tick: {}\n tick_spacing: {}\n sqrt_price_current: {}\n sqrt_price_max: {}\n{}",
        //         swap_max,
        //         reserve_current,
        //         reserve_max,
        //         self.tick,
        //         tick_spacing,
        //         q_64_96_to_decimal(self.sqrt_price_x96),
        //         q_64_96_to_decimal(sqrt_price_max_x96),
        //         e
        //     )
        // })
        reserve_max
            .checked_sub(reserve_current)
            .ok_or(
                anyhow!(
                    "Failed to calculate swap limit y (subtraction caused overflow): reserve_max: {}\n reserve_current: {}\n tick: {}\n tick_spacing: {}\n sqrt_price_current: {}\n sqrt_price_max_x96: {}",
                    reserve_max,
                    reserve_current,
                    self.tick,
                    tick_spacing,
                    q_64_96_to_decimal(self.sqrt_price_x96),
                    q_64_96_to_decimal(sqrt_price_max_x96),
                )
            )
    }

    // pub fn pool_virtual_reserves(
    //     &self,
    //     token_0_decimals: u32,
    //     token_1_decimals: u32,
    //     fee: u32,
    //     tick_spacing: u32,
    // ) -> Result<PoolVirtualReserves> {
    //     let reserve_x = self.virtual_reserve_x()?;
    //     let reserve_y = self.virtual_reserve_y()?;

    //     let max_swap_x = self.swap_limit_x(tick_spacing)?;
    //     let max_swap_y = self.swap_limit_y(tick_spacing)?;

    //     Ok(PoolVirtualReserves {
    //         token_0: u128_to_decimal(reserve_x, token_0_decimals).and_then(|t0| {
    //             t0.to_f64()
    //                 .ok_or(anyhow!("Failed to convert token_0 to f64"))
    //         })?,
    //         token_1: u128_to_decimal(reserve_y, token_1_decimals).and_then(|t1| {
    //             t1.to_f64()
    //                 .ok_or(anyhow!("Failed to convert token_1 to f64"))
    //         })?,
    //         fee_multiplier: (dec!(1.0) - fee_amount_from_int(fee))
    //             .to_f64()
    //             .ok_or(anyhow!("Failed to convert fee multiplier to f64"))?,
    //         max_swap_0: u128_to_decimal(max_swap_x, token_0_decimals).and_then(|t0| {
    //             t0.to_f64()
    //                 .ok_or(anyhow!("Failed to convert max_swap_0 to f64"))
    //         })?,
    //         max_swap_1: u128_to_decimal(max_swap_y, token_1_decimals).and_then(|t1| {
    //             t1.to_f64()
    //                 .ok_or(anyhow!("Failed to convert max_swap_1 to f64"))
    //         })?,
    //     })
    // }

    // pub fn calculate_quote(&self, amount_in: u128, fee: Fee, is_reverse_swap: bool) -> u128 {
    //     let sqrt_price_x96 = U256::from(self.sqrt_price_x96);
    //     let liquidity = U256::from(self.liquidity);
    //     let q_96 = U256::from(1u128 << 96);

    //     // Apply fee to amount_in
    //     let fee_pips = fee as u32;

    //     let amount_in_minus_fee = U256::from(amount_in)
    //         .checked_mul(U256::from(1_000_000 - fee_pips))
    //         .unwrap()
    //         .checked_div(U256::from(1_000_000))
    //         .unwrap();

    //     if !is_reverse_swap {
    //         // tonken_0 to token_1
    //         let numerator = liquidity << 96;

    //         let new_sqrt_price_x96 = numerator / (numerator / sqrt_price_x96 + amount_in_minus_fee);
    //         let token_1_delta = liquidity * (sqrt_price_x96 - new_sqrt_price_x96) / q_96;

    //         u128::try_from(token_1_delta).unwrap()
    //     } else {
    //         panic!("Reverse swap not implemented yet");
    //     }
    //     // Return the final amount out, ensuring it fits in u128
    //     // u128::try_from(amount_out).unwrap()
    // }

    // pub fn calculate_quote_dec(
    //     &self,
    //     amount_in: Decimal,
    //     fee: Fee,
    //     is_reverse_swap: bool,
    //     token_0_decimals: u32,
    //     token_1_decimals: u32,
    // ) -> Decimal {
    //     let amount_in_minus_fee = amount_in * (dec!(1.0) - fee.fee_amount());
    //     let reserve_x = Decimal::from_i128_with_scale(self.virtual_reserve_x().unwrap() as i128, 0)
    //         / dec!(10).powi(token_0_decimals as i64);
    //     let reserve_y = Decimal::from_i128_with_scale(self.virtual_reserve_y().unwrap() as i128, 0)
    //         / dec!(10).powi(token_1_decimals as i64);

    //     if !is_reverse_swap {
    //         reserve_y * amount_in_minus_fee / (reserve_x + amount_in_minus_fee)
    //     } else {
    //         reserve_x * amount_in_minus_fee / (reserve_y + amount_in_minus_fee)
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;

    const POOL_STATE_WBTC_USDC: PoolState = PoolState {
        sqrt_price_x96: U160::from_limbs([17134602959287796597, 139272449984, 0]),
        liquidity: 50170120777514,
        tick: I24::from_limbs([69583]),
    };

    #[test]
    fn test_max_swap_x() {
        let pool_state = POOL_STATE_WBTC_USDC;

        let max_swap_x = pool_state.swap_limit_x(60).unwrap();
        let virtual_reserve_x = pool_state.virtual_reserve_x(); //.unwrap();

        // println!("Max Swap X: {:?}", max_swap_x);
        // println!("Virtual Reserve X: {:?}", virtual_reserve_x);

        assert!(
            max_swap_x < virtual_reserve_x,
            "Max swap X exceeds virtual reserve X"
        );
        assert!(max_swap_x > 0, "Max swap X should be greater than zero");
    }

    #[test]
    fn test_max_swap_x_1() {
        let pool_state = PoolState {
            sqrt_price_x96: U160::from_limbs([5277553418330626170, 83406331270155, 0]),
            liquidity: 4844714101140627498,
            tick: I24::from_limbs([197490]),
        };

        let max_swap_x = pool_state.swap_limit_x(60);

        assert!(
            max_swap_x.is_ok(),
            "Failed to calculate max swap X: {:?}",
            max_swap_x.err()
        );
    }

    #[test]
    fn test_max_swap_y() {
        let pool_state = POOL_STATE_WBTC_USDC;

        let max_swap_y = pool_state.swap_limit_y(60).unwrap();
        let virtual_reserve_y = pool_state.virtual_reserve_y(); // .unwrap();

        println!("Max Swap Y: {:?}", max_swap_y);
        println!("Virtual Reserve Y: {:?}", virtual_reserve_y);

        assert!(
            max_swap_y < virtual_reserve_y,
            "Max swap Y exceeds virtual reserve Y"
        );
    }

    #[test]
    fn test_decimal_liquidity_check() {
        let pool_state = POOL_STATE_WBTC_USDC;

        let reserves_x: u128 = pool_state.virtual_reserve_x().try_into().unwrap();

        let reserves_y: u128 = pool_state.virtual_reserve_y().try_into().unwrap();

        let sqrt_price = q_64_96_to_decimal(pool_state.sqrt_price_x96);
        let liquidity_alt = (reserves_x * reserves_y).isqrt();
        println!("Liquidity: {}", pool_state.liquidity);
        println!("Liquidity alt: {}", liquidity_alt);
        let reserves_x_alt = Decimal::from_i128_with_scale(liquidity_alt as i128, 0) / sqrt_price;
        let reserves_y_alt = Decimal::from_i128_with_scale(liquidity_alt as i128, 0) * sqrt_price;

        // let dec_liquidity = Decimal::sqrt(&(reserves_x * reserves_y)).unwrap();
        // println!("{}", dec_liquidity);
        // let dec_sqrt_price = q_64_96_to_decimal(pool_state.sqrt_price_x96);
        // println!("{}", dec_sqrt_price);

        // let dec_reserves_x = dec_liquidity * dec_sqrt_price;
        // let dec_reserves_y = dec_liquidity / dec_sqrt_price;

        println!("X: {} - {}", reserves_x, reserves_x_alt);
        println!("Y: {} - {}", reserves_y, reserves_y_alt);
    }

    // #[test]
    // fn test_decimal_quote() {
    //     let pool_state = POOL_STATE_WBTC_USDC;
    //     let fee = Fee::Medium;
    //     println!("Reserves X: {:?}", pool_state.virtual_reserve_x());
    //     println!("Reserves Y: {:?}", pool_state.virtual_reserve_y());

    //     let amount_in = 10u128.pow(tokens::WBTC.decimals());
    //     let expected_amount_out = Decimal::from_i128_with_scale(
    //         pool_state.calculate_quote(
    //             amount_in,
    //             tokens::WBTC.decimals,
    //             tokens::USDC.decimals,
    //             fee,
    //             false,
    //         ) as i128,
    //         tokens::USDC.decimals(),
    //     );

    //     let dec_amount_in = Decimal::new(1, 0);
    //     let dec_amount_out = pool_state.calculate_quote_dec(
    //         dec_amount_in,
    //         false,
    //         tokens::WBTC.decimals(),
    //         tokens::USDC.decimals(),
    //     );

    //     println!("Expected Amount Out: {:?}", expected_amount_out);
    //     println!("Decimal Amount Out: {:?}", dec_amount_out);

    //     assert!(
    //         (expected_amount_out - dec_amount_out).abs() < dec!(0.001),
    //         "Expected: {}, Actual: {}",
    //         expected_amount_out,
    //         dec_amount_out
    //     );
    // }
}
