use alloy::primitives::U160;
use anyhow::{Result, anyhow};
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use serde::Deserialize;

const DENOMINATOR_Q96: Decimal = dec!(7.92281625e28);

pub fn q_64_96_to_decimal(q_64_96: U160) -> Decimal {
    let q_64_96_bytes = q_64_96.to_le_bytes::<20>();
    // In ethereum Big-endian is only used for bytes and string types
    // Little-endian is used for every other type of variable. Some examples are: uint8, uint32, uint256, int8, boolean, address, etc…
    // The sqrt price is stored as a U160. First 96 bits are the fractional part, the last 64 bits are the integer part.

    // even though we only need 12 bytes for the fractional part (12 bytes * 8 = 96 bits)
    // it will be copied into an i128 to make it easier to convert to a Decimal
    let mut q_64_96_frac_bytes = [0u8; 16];

    // 8 bytes for the integer part for to store the value in a u64
    let mut q_64_96_int_bytes = [0u8; 8];

    // println!("q_64_96_bytes: {:?}", q_64_96_bytes);

    // First 96 bits are the fractional part
    q_64_96_frac_bytes[..12].copy_from_slice(&q_64_96_bytes[..12]);

    // Last 64 bits are the integer part
    q_64_96_int_bytes.copy_from_slice(&q_64_96_bytes[12..20]);

    // println!("q_64_96_frac_bytes: {:?}", q_64_96_frac_bytes);
    // println!("q_64_96_int_bytes: {:?}", q_64_96_int_bytes);

    // conversion panics if `scale` is > 28 or if `num` exceeds the maximum supported 96 bits
    // here it's guaranteed that the scale is 0 (hard-coded here) and the number is at most 96 bits (only 12 bytes copied in the code above)
    let sqrt_price_frac =
        Decimal::from_i128_with_scale(i128::from_le_bytes(q_64_96_frac_bytes), 0) / DENOMINATOR_Q96;

    // println!("sqrt_price_frac: {:?}", sqrt_price_frac);

    let sqrt_price_int: Decimal = u64::from_le_bytes(q_64_96_int_bytes).into();

    // println!("sqrt_price_int: {:?}", sqrt_price_int);

    let sqrt_price = sqrt_price_frac + sqrt_price_int;

    sqrt_price
}

pub fn decimal_to_q_64_96(decimal: Decimal) -> Result<U160> {
    let scale_multiplier = 10i128.pow(decimal.scale());
    let integer_part = decimal.mantissa() / scale_multiplier;

    if integer_part > u64::MAX as i128 {
        return Err(anyhow!("Integer part exceeds 64 bits: {}", integer_part));
    }

    let frac = (decimal.fract() * DENOMINATOR_Q96).round().mantissa();
    let mut frac_bytes: [u8; 20] = [0; 20];
    frac_bytes[..12].copy_from_slice(&frac.to_le_bytes()[..12]);
    frac_bytes[12..20].copy_from_slice(&integer_part.to_le_bytes()[..8]);

    // println!("scale: {}", decimal.scale());
    // println!("scale multiplier: {}", scale_multiplier);
    // println!("integer part: {}", integer_part);
    // println!("frac: {} - {:?}", frac, frac.to_le_bytes());
    // println!("frac bytes: {:?}", frac_bytes);

    Ok(U160::from_le_bytes::<20>(frac_bytes))
}

pub fn tick_from_str<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt_str: Option<String> = Option::deserialize(deserializer)?;

    match opt_str {
        Some(str) => serde_json::from_str(str.as_str()).map_err(serde::de::Error::custom),
        None => Ok(0),
    }
}

pub fn fee_amount_from_int(fee: u32) -> Decimal {
    Decimal::new(fee as i64, 6)
}

#[cfg(test)]
pub mod tests {

    use crate::uniswap_internal::v3::pool_state::PoolState;

    use super::*;
    use alloy::primitives::{FixedBytes, U128, U256, aliases::I24, bytes, fixed_bytes};
    use proptest::prelude::*;
    use rust_decimal_macros::dec;

    pub const POOL_STATE_WBTC_USDC: PoolState = PoolState {
        sqrt_price_x96: U160::from_limbs([600426521302246757, 143091961054, 0]),
        liquidity: 33276313374575,
        tick: I24::from_limbs([70124]),
    };

    pub const POOL_STATE_USDC_WETH: PoolState = PoolState {
        sqrt_price_x96: U160::from_limbs([10065235132669518444, 83425719956160, 0]),
        liquidity: 7432282805924149038,
        tick: I24::from_limbs([197495]),
    };

    pub const POOL_STATE_WBTC_WETH: PoolState = PoolState {
        sqrt_price_x96: U160::from_limbs([6853182935233074302, 2780479044364478, 0]),
        liquidity: 34788459374073205,
        tick: I24::from_limbs([267627]),
    };

    // pub fn make_pool_reserves_map()
    // -> HashMap<(TokenAddress, TokenAddress, Fee), PoolVirtualReserves> {
    //     let pool_reserves_wbtc_usdc = POOL_STATE_WBTC_USDC
    //         .pool_virtual_reserves(
    //             tokens::WBTC.decimals,
    //             tokens::USDC.decimals,
    //             Fee::Medium as u32,
    //             Fee::Medium.tick_spacing(),
    //         )
    //         .unwrap();

    //     let pool_reserves_usdc_weth = POOL_STATE_USDC_WETH
    //         .pool_virtual_reserves(
    //             tokens::USDC.decimals,
    //             tokens::WETH.decimals,
    //             Fee::Medium as u32,
    //             Fee::Medium.tick_spacing(),
    //         )
    //         .unwrap();

    //     let pool_reserves_wbtc_weth = POOL_STATE_WBTC_WETH
    //         .pool_virtual_reserves(
    //             tokens::WBTC.decimals,
    //             tokens::WETH.decimals,
    //             Fee::Medium as u32,
    //             Fee::Medium.tick_spacing(),
    //         )
    //         .unwrap();

    //     HashMap::from([
    //         (
    //             (tokens::USDC.address(), tokens::WBTC.address(), Fee::Medium),
    //             pool_reserves_wbtc_usdc.clone().inverse(),
    //         ),
    //         (
    //             (tokens::WBTC.address(), tokens::USDC.address(), Fee::Medium),
    //             pool_reserves_wbtc_usdc.clone(),
    //         ),
    //         (
    //             (tokens::WETH.address(), tokens::WBTC.address(), Fee::Medium),
    //             pool_reserves_wbtc_weth.clone().inverse(),
    //         ),
    //         (
    //             (tokens::WBTC.address(), tokens::WETH.address(), Fee::Medium),
    //             pool_reserves_wbtc_weth.clone(),
    //         ),
    //         (
    //             (tokens::WETH.address(), tokens::USDC.address(), Fee::Medium),
    //             pool_reserves_usdc_weth.clone().inverse(),
    //         ),
    //         (
    //             (tokens::USDC.address(), tokens::WETH.address(), Fee::Medium),
    //             pool_reserves_usdc_weth.clone(),
    //         ),
    //     ])
    // }

    #[test]
    fn test_deserialize_tick() {
        let json = r#""123456""#;
        let mut deserializer = serde_json::Deserializer::from_str(json);

        let tick = tick_from_str(&mut deserializer);

        assert_eq!(tick.unwrap(), 123456);
    }

    #[test]
    fn test_deserialize_tick_null() {
        let json_null = r"null";
        let mut deserializer = serde_json::Deserializer::from_str(json_null);

        let tick = tick_from_str(&mut deserializer);

        assert_eq!(tick.unwrap(), 0);
    }

    #[test]
    fn test_deserialize_tick_invalid() {
        let json_invalid = r#""invalid""#;
        let mut deserializer = serde_json::Deserializer::from_str(json_invalid);

        let result = tick_from_str(&mut deserializer);
        assert!(result.is_err());
    }

    #[test]
    fn test_q64_96_decimal_roundtrip() {
        let original_decimal = dec!(123456789.987654321);

        let q_64_96 = decimal_to_q_64_96(original_decimal).unwrap();

        let converted_decimal = q_64_96_to_decimal(q_64_96);

        assert_eq!(original_decimal, converted_decimal);
    }

    #[test]
    fn test_q64_96_decimal_roundtrip_1() {
        let original_decimal = dec!(0.1);

        let q_64_96 = decimal_to_q_64_96(original_decimal).unwrap();

        let converted_decimal = q_64_96_to_decimal(q_64_96);

        assert_eq!(original_decimal, converted_decimal);
    }

    #[test]
    fn test_q64_96_decimal_roundtrip_2() {
        let original_decimal = dec!(0.000022443076029480861);
        let q_64_96 = decimal_to_q_64_96(original_decimal).unwrap();

        let converted_decimal = q_64_96_to_decimal(q_64_96);

        assert_eq!(original_decimal, converted_decimal);
    }

    #[test]
    fn u256_byte_layout_example() {
        let b = bytes!("0xfff97272373d413259a46990580e213a");
        let mut bs: [u8; 32] = [0; 32];

        bs[16..32].copy_from_slice(&b);

        let u256_expected = U256::from_be_bytes::<32>(bs);

        let u256_value = U256::from_limbs([
            0x59a46990580e213a,
            0xfff97272373d4132,
            0x0000000000000000,
            0x0000000000000000,
        ]);

        let fb: FixedBytes<16> = fixed_bytes!("0xfff97272373d413259a46990580e213a");
        let u256_value_2 = U256::from(U128::from_be_bytes(fb.0));

        // let fb: FixedBytes<16> = fixed_bytes!("0xfff97272373d413259a46990580e213a");

        println!("u256_value: {:?}", u256_value.as_limbs());
        println!("u256_expected: {:?}", u256_expected.as_limbs());
        println!("u256_value_2: {:?}", u256_value_2.as_limbs());
        assert_eq!(u256_value, u256_expected);
        assert_eq!(u256_value, u256_value_2);
    }

    proptest! {
        #[test]
        fn proptest_q64_96_decimal_roundtrip(mantissa: u64, exponent in 0..28u32) {
            // max integer part should not exceed 64 bits
            let original_decimal = Decimal::from_i128_with_scale(mantissa as i128, exponent);

            let q_64_96 = decimal_to_q_64_96(original_decimal).unwrap();
            let converted_decimal = q_64_96_to_decimal(q_64_96);

            prop_assert_eq!(original_decimal, converted_decimal);
        }

        #[test]
        fn proptest_q64_96_decimal_roundtrip_max_bits(exponent in 10..28u32) {
            let frac = (1u128 << 96) - 1;
            let original_decimal = Decimal::from_i128_with_scale(frac as i128, exponent);

            let q_64_96 = decimal_to_q_64_96(original_decimal).unwrap();

            let converted_decimal = q_64_96_to_decimal(q_64_96);

            assert_eq!(original_decimal, converted_decimal);
        }
    }
}
