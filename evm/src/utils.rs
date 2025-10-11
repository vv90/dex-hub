use std::{
    fmt::Debug,
    ops::{Div, Rem},
};

use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug)]
pub enum DecimalConversionError<T>
where
    T: Debug + TryFrom<u128> + TryInto<i128>,
    <T as TryInto<i128>>::Error: Debug,
    <T as TryFrom<u128>>::Error: Debug,
{
    ExponentiationOverflow(T, u32),
    IntegerPartOverflow(T),
    ConversionFromError(<T as TryFrom<u128>>::Error),
    ConversionIntoError(<T as TryInto<i128>>::Error),
}

impl<T> std::error::Error for DecimalConversionError<T>
where
    T: Debug + TryFrom<u128> + TryInto<i128>,
    <T as TryInto<i128>>::Error: Debug,
    <T as TryFrom<u128>>::Error: Debug,
{
}

impl<T> std::fmt::Display for DecimalConversionError<T>
where
    T: Debug + TryFrom<u128> + TryInto<i128>,
    <T as TryFrom<u128>>::Error: Debug,
    <T as TryInto<i128>>::Error: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecimalConversionError::ExponentiationOverflow(value, decimals) => {
                write!(
                    f,
                    "exponentiation overflow for value {:?} and decimals {}",
                    value, decimals
                )
            }
            DecimalConversionError::IntegerPartOverflow(value) => {
                write!(f, "integer part overflow for value {:?}", value)
            }
            DecimalConversionError::ConversionFromError(err) => {
                write!(f, "conversion from error: {:?}", err)
            }
            DecimalConversionError::ConversionIntoError(err) => {
                write!(f, "conversion into error: {:?}", err)
            }
        }
    }
}

pub fn try_into_decimal<T>(value: T, decimals: u32) -> Result<Decimal, DecimalConversionError<T>>
where
    T: Div<Output = T>
        + Rem<Output = T>
        + PartialOrd
        + TryFrom<u128>
        + TryInto<i128>
        + Copy
        + Debug,
    <T as TryFrom<u128>>::Error: Debug,
    <T as TryInto<i128>>::Error: Debug,
{
    let denominator = 10u128
        .checked_pow(decimals)
        .ok_or(DecimalConversionError::ExponentiationOverflow(
            value, decimals,
        ))
        .and_then(|x| {
            x.try_into()
                .map_err(DecimalConversionError::ConversionFromError)
        })?;

    let quotient: T = value / denominator;
    let remainder: T = value % denominator;

    let max_quotient =
        T::try_from(1 << 96u128).map_err(DecimalConversionError::ConversionFromError)?;

    // check that quotient does not exceed 96 bits
    if quotient >= max_quotient {
        Err(DecimalConversionError::IntegerPartOverflow(quotient))
    } else {
        let q: i128 = quotient
            .try_into()
            .map_err(DecimalConversionError::ConversionIntoError)?;
        let r: i128 = remainder
            .try_into()
            .map_err(DecimalConversionError::ConversionIntoError)?;

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
