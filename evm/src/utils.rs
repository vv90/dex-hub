use std::{
    fmt::Debug,
    ops::{Div, Rem},
    panic::catch_unwind,
};

use anyhow::{Result, anyhow};
use rust_decimal::Decimal;
use serde::Deserialize;

pub fn try_into_decimal<
    T: Div<Output = T> + Rem<Output = T> + PartialOrd + TryFrom<u128> + TryInto<i128> + Copy + Debug,
>(
    value: T,
    decimals: u32,
) -> Result<Decimal> {
    let denominator = catch_unwind(|| 10u128.pow(decimals))
        .map_err(|_| anyhow!("Failed to calcurate 10^{}: Multiply overflow.", decimals))
        .and_then(|x| {
            x.try_into()
                .map_err(|_| anyhow!("Failed to convert denominator '{}' to value type ", x))
        })?;

    let quotient: T = value / denominator;
    let remainder: T = value % denominator;

    let max_quotient = T::try_from(1 << 96u128).map_err(|_| ()).unwrap();

    // check that quotient does not exceed 96 bits
    if quotient >= max_quotient {
        Err(anyhow!("Integer part exceeds 96 bits: {:?}", value))
    } else {
        let q: i128 = quotient
            .try_into()
            .map_err(|_| anyhow!("unable to convert value {:?} to i128", quotient))?;
        let r: i128 = remainder
            .try_into()
            .map_err(|_| anyhow!("unable to convert value {:?} to i128", remainder))?;

        let integer_part = Decimal::from_i128_with_scale(q, 0);
        let decimal_part = Decimal::from_i128_with_scale(r, decimals);

        Ok(integer_part + decimal_part)
    }
}

pub fn u32_from_str<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    serde_json::from_str(s.as_str()).map_err(serde::de::Error::custom)
}
