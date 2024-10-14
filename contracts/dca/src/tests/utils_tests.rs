use std::str::FromStr;
use crate::error::ContractError;
use test_case::test_case;
use crate::utils::{normalize_price, price_to_tick_index};
use cosmwasm_std::{Decimal, Int128};



#[test_case(Decimal::from_str("123456791234567.000000000000000000").unwrap() => -324485; "large positive number with decimals")]
#[test_case(Decimal::from_str("123456791234567").unwrap() => -324485; "large positive number without decimals")]
#[test_case(Decimal::from_str("12345").unwrap() => -94215; "medium positive number")]
#[test_case(Decimal::from_str("11.0").unwrap() => -23980; "small positive number greater than 1")]
#[test_case(Decimal::from_str("2.0").unwrap() => -6932; "number 2")]
#[test_case(Decimal::from_str("1.10").unwrap() => -953; "slightly above 1")]
#[test_case(Decimal::from_str("1.0").unwrap() => 0; "exactly 1")]
#[test_case(Decimal::from_str("0.9").unwrap() => 1054; "slightly below 1")]
#[test_case(Decimal::from_str("0.5").unwrap() => 6932; "0.5")]
#[test_case(Decimal::from_str("0.1").unwrap() => 23027; "0.1")]
#[test_case(Decimal::from_str("0.01").unwrap() => 46054; "0.01")]
#[test_case(Decimal::from_str("0.0011").unwrap() => 68128; "small fraction")]
#[test_case(Decimal::from_str("0.000123").unwrap() => 90038; "smaller fraction")]
#[test_case(Decimal::from_str("0.00000009234").unwrap() => 161986; "tiny fraction")]
#[test_case(Decimal::from_str("0.000000000000123").unwrap() => 297281; "tinier fraction")]
fn test_price_to_tick_index(price: Decimal) -> i64 {
    price_to_tick_index(price).unwrap()
}

#[test_case(Decimal::zero() => Err(ContractError::InvalidPrice); "zero price")]
fn test_price_to_tick_index_error(price: Decimal) -> Result<i64, ContractError> {
    price_to_tick_index(price)
}

#[test_case(Int128::new(1234567), 6 => Ok(Decimal::from_str("1.234567").unwrap()); "positive number with 6 decimals")]
#[test_case(Int128::new(1234567), 2 => Ok(Decimal::from_str("12345.67").unwrap()); "positive number with 2 decimals")]
#[test_case(Int128::new(1234567), 0 => Ok(Decimal::from_str("1234567").unwrap()); "positive number with 0 decimals")]
#[test_case(Int128::new(1234567890098764321), 12 => Ok(Decimal::from_str("1234567.890098764321").unwrap()); "large positive number")]
#[test_case(Int128::zero(), 6 => Ok(Decimal::zero()); "zero")]
#[test_case(Int128::new(-1234567), 6 => Err(ContractError::PriceIsNegative); "negative number")]
fn test_normalize_price(
    input_price: Int128,
    input_decimals: u64,
) -> Result<Decimal, ContractError> {
    normalize_price(input_price, input_decimals)
}
