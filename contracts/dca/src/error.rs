use cosmwasm_std::{OverflowError, StdError, Uint128};
use thiserror::Error;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("field {kind} should not be empty")]
    EmptyValue { kind: String },

    #[error("only accepts 1 token to be sold")]
    MultipleFundsSent,

    #[error("already at max schedule capacity")]
    MaxSchedulesReached,

    #[error("overflow has happened")]
    OverflowError(#[from] OverflowError),

    #[error("User has an active schedule already.")]
    ExistingBalance,

    #[error("Failed to decode response data")]
    DecodingError,

    #[error("No response data from place limit order")]
    NoResponseData,

    #[error("Expected Schadule, but not found")]
    ScheduleNotFound,

    #[error("denom {denom} is not a correct IBC denom: {reason}")]
    InvalidIbcDenom { denom: String, reason: String },

    #[error("base: {base} quote: {quote} is not a valid currency pair: {reason}")]
    InvalidCurrencyPair {
        base: String,
        quote: String,
        reason: String,
    },

    #[error(
        "limit order execution used: {requested} usd, but owner only has: {available} available"
    )]
    InsufficientLiquidity {
        requested: Uint128,
        available: Uint128,
    },

    #[error("Market {symbol}, {quote} not found in {location}")]
    UnsupportedMarket {
        symbol: String,
        quote: String,
        location: String,
    },

    #[error("Market {symbol}, {quote} not enabled in {location}")]
    DisabledMarket {
        symbol: String,
        quote: String,
        location: String,
    },

    #[error("Market {symbol}, {quote} did not return an block height")]
    PriceNotAvailable { symbol: String, quote: String },

    #[error("Market {symbol}, {quote} returned a nil price")]
    PriceIsNil { symbol: String, quote: String },

    #[error("Market {symbol}, {quote} is older than {max_blocks} blocks")]
    PriceTooOld {
        symbol: String,
        quote: String,
        max_blocks: u64,
    },

    #[error("input for {input} is invalid: {reason}")]
    MalformedInput { input: String, reason: String },

    #[error("Only USD quote currency supported. Quote Currencies provided: {quote0}, {quote1}")]
    OnlySupportUsdQuote { quote0: String, quote1: String },

    #[error("Invalid DEX deposit base fee: {fee}")]
    InvalidBaseFee { fee: u64 },

    #[error("Invalid deposit percentage: {percentage}. Normal range is [0-100]")]
    InvalidDepositPercentage { percentage: u64 },

    #[error("Too many decimals from oracle responce, exceeds u32 allowance")]
    TooManyDecimals,

    #[error("Price cannot be negative")]
    PriceIsNegative,

    #[error("Failed to convert value to Decimal")]
    DecimalConversionError,

    #[error("Failed to devide decimal")]
    DecimalDivisionError,

    #[error("No funds sent with deposit function")]
    NoFundsSent,

    #[error("Attempted deposit of invalid token")]
    InvalidToken,

    #[error("Msg sender must be the contract owner")]
    Unauthorized,

    #[error("No funds available")]
    NoFundsAvailable,

    #[error("Funds cannot be received here")]
    FundsNotAllowed,

    #[error("failed to convert uint to int. value of coin amount as Uint128 exceeds max possible Int128 amount")]
    ConversionError,

    #[error("Price is invalid")]
    InvalidPrice,

    #[error("Insufficient balance for Deposit: available: {available}, required: {required}")]
    InsufficientFunds {
        available: Uint128,
        required: Uint128,
    },

    #[error("Liquidity exists but tick index was not returned")]
    TickIndexDoesNotExist,

    #[error("Liquidity exists but cannot be retreived")]
    LiquidityNotFound,
}
