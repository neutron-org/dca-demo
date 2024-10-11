use std::str::FromStr;

use crate::error::ContractError;
use crate::msg::{
    CombinedPriceResponse, DepositResult, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use crate::state::{Config, PairData, Balances, CONFIG};
use cosmwasm_std::{
    attr, entry_point, to_json_binary, BalanceResponse, BankQuery, Binary, Coin, CosmosMsg,
    Decimal, Deps, DepsMut, Env, Fraction, Int128, MessageInfo, QueryRequest, Response, StdResult,
    Uint128, Uint64, Addr,
};
use cw2::set_contract_version;
use serde::{Deserialize, Serialize};

pub type ContractResult<T> = core::result::Result<T, ContractError>;

use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};
use neutron_std::types::slinky::marketmap::v1::{
    MarketMap, MarketMapRequest, MarketMapResponse, MarketRequest, MarketResponse,
};

use neutron_std::types::slinky::oracle::v1::{
    GetAllCurrencyPairsRequest, GetAllCurrencyPairsResponse, GetPriceRequest, GetPriceResponse,
};
use neutron_std::types::slinky::types::v1::CurrencyPair;
use neutron_std::types::slinky::oracle::v1::OracleQuerier;
use neutron_std::types::slinky::marketmap::v1::MarketmapQuerier;

pub fn load_config(deps: Deps<NeutronQuery>) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

pub fn get_pair_id_str(token0: &str, token1: &str) -> (String) {
    let mut tokens = [token0.clone(), token1.clone()];
    if token1 < token0 {
        tokens.reverse();
    }
    [tokens[0].clone(), tokens[1].clone()].join("<>")
}

pub fn query_oracle_price(
    deps: &Deps<NeutronQuery>,
    pair: &CurrencyPair,
) -> ContractResult<GetPriceResponse> {
    let querier = OracleQuerier::new(&deps.querier);
    let price: GetPriceResponse = querier.get_price(Some(pair.clone()))?;
    Ok(price)
}

pub fn query_marketmap_market(
    deps: &Deps<NeutronQuery>,
    pair: &CurrencyPair,
) -> ContractResult<MarketResponse> {
    let querier = MarketmapQuerier::new(&deps.querier);
    let market_response: MarketResponse = querier.market(Some(pair.clone()))?;
    Ok(market_response)
}

pub fn query_oracle_currency_pairs(deps: &Deps<NeutronQuery>) -> ContractResult<Vec<CurrencyPair>> {
    let querier = OracleQuerier::new(&deps.querier);
    let oracle_currency_pairs_response: GetAllCurrencyPairsResponse = querier.get_all_currency_pairs()?;
    Ok(oracle_currency_pairs_response.currency_pairs)
}

pub fn query_marketmap_market_map(deps: &Deps<NeutronQuery>) -> ContractResult<MarketMap> {
    let querier = MarketmapQuerier::new(&deps.querier);
    let marketmap_currency_pairs_response = querier.market_map()?;
    Ok(marketmap_currency_pairs_response.market_map.unwrap())
}

pub fn validate_market(
    deps: &Deps<NeutronQuery>,
    env: &Env,
    pair: &CurrencyPair,
    max_blocks_old: u64,
) -> ContractResult<Response> {
    // get price response here to avoid querying twice on recent and not_nil checks
    let price_response = query_oracle_price(deps, pair)?;
    validate_market_supported_xoracle(deps, &pair, None)?;
    validate_market_supported_xmarketmap(deps, &pair, None)?;
    // validate_market_enabled(deps, &pair, None)?;
    validate_price_recent(
        deps,
        env,
        &pair,
        max_blocks_old,
        Some(price_response.clone()),
    )?;
    validate_price_not_nil(deps, &pair, Some(price_response.clone()))?;
    Ok(Response::new())
}

pub fn validate_price_recent(
    deps: &Deps<NeutronQuery>,
    env: &Env,
    pair: &CurrencyPair,
    max_blocks_old: u64,
    oracle_price_response: Option<GetPriceResponse>,
) -> ContractResult<Response> {
    let current_block_height: u64 = env.block.height;
    let oracle_price_response = match oracle_price_response {
        Some(response) => response,
        None => query_oracle_price(deps, &pair)?,
    };

    let price: neutron_std::types::slinky::oracle::v1::QuotePrice = oracle_price_response
        .price
        .ok_or_else(|| ContractError::PriceNotAvailable {
            symbol: pair.base.clone(),
            quote: pair.quote.clone(),
        })?;
    if (current_block_height - price.block_height) > max_blocks_old {
        return Err(ContractError::PriceTooOld {
            symbol: pair.base.clone(),
            quote: pair.quote.clone(),
            max_blocks: max_blocks_old,
        });
    }

    Ok(Response::new())
}

pub fn validate_market_enabled(
    deps: &Deps<NeutronQuery>,
    pair: &CurrencyPair,
    marketmap_market_response: Option<MarketResponse>,
) -> ContractResult<Response> {
    let marketmap_market_response: MarketResponse = match marketmap_market_response {
        Some(response) => response,
        None => query_marketmap_market(deps, &pair)?,
    };

    if let Some(market) = marketmap_market_response.market {
        if let Some(ticker) = market.ticker {
            if !ticker.enabled {
                return Err(ContractError::UnsupportedMarket {
                    symbol: pair.base.clone(),
                    quote: pair.quote.clone(),
                    location: "x/marketmap".to_string(),
                });
            }
        }
    }
    Ok(Response::new())
}

pub fn validate_market_supported_xoracle(
    deps: &Deps<NeutronQuery>,
    pair: &CurrencyPair,
    oracle_currency_pairs: Option<Vec<CurrencyPair>>,
) -> ContractResult<Response> {
    let supported_pairs = match oracle_currency_pairs {
        Some(pairs) => pairs,
        None => query_oracle_currency_pairs(deps)?,
    };

    if !supported_pairs.contains(pair) {
        return Err(ContractError::UnsupportedMarket {
            symbol: pair.base.clone(),
            quote: pair.quote.clone(),
            location: "x/oracle".to_string(),
        });
    }

    Ok(Response::new())
}

pub fn validate_market_supported_xmarketmap(
    deps: &Deps<NeutronQuery>,
    pair: &CurrencyPair,
    market_map: Option<MarketMap>,
) -> ContractResult<Response> {
    let map = match market_map {
        Some(map) => map,
        None => query_marketmap_market_map(deps)?,
    };
    let key: String = format!("{}/{}", pair.base, pair.quote);
    if map.markets.contains_key(&key) == false {
        return Err(ContractError::UnsupportedMarket {
            symbol: pair.base.clone(),
            quote: pair.quote.clone(),
            location: "x/marketmap".to_string(),
        });
    }

    Ok(Response::new())
}

pub fn validate_price_not_nil(
    deps: &Deps<NeutronQuery>,
    pair: &CurrencyPair,
    oracle_price_response: Option<GetPriceResponse>,
) -> ContractResult<Response> {
    let oracle_price_response = match oracle_price_response {
        Some(response) => response,
        None => query_oracle_price(deps, &pair)?,
    };

    if oracle_price_response.nonce == 0 {
        return Err(ContractError::PriceIsNil {
            symbol: pair.base.clone(),
            quote: pair.quote.clone(),
        });
    }
    Ok(Response::new())
}

// Get price of NTRN in USD
pub fn get_price(deps: Deps<NeutronQuery>, env: Env) -> ContractResult<Decimal> {
    let config = load_config(deps)?;
    let pair: CurrencyPair = config.pair_data.currency_pair;

    // Query the oracle for the price
    let price_response: GetPriceResponse = query_oracle_price(&deps, &pair)?;
    validate_price_not_nil(&deps, &pair, Some(price_response.clone()))?;
    validate_price_recent(
        &deps,
        &env,
        &pair,
        config.max_blocks_old,
        Some(price_response.clone()),
    )?;

    // Parse the price string to Int128 and normalize
    let price_int128 = Int128::from_str(&price_response.price.unwrap().price)
        .map_err(|_| ContractError::InvalidPrice)?;
    let price = normalize_price(price_int128, price_response.decimals)?;

    Ok(price)
}

pub fn normalize_price(price: Int128, decimals: u64) -> ContractResult<Decimal> {
    // Ensure decimals does not exceed u32::MAX
    if decimals > u32::MAX as u64 {
        return Err(ContractError::TooManyDecimals);
    }
    if price < Int128::zero() {
        return Err(ContractError::PriceIsNegative);
    }
    let abs_value: u128 = price.i128().abs() as u128;
    Decimal::from_atomics(abs_value, decimals as u32)
        .map_err(|e| ContractError::DecimalConversionError)
}

pub fn get_user_balance(
    deps: &DepsMut<NeutronQuery>,
    denom: String,
    owner: Addr,
) -> Result<Uint128, ContractError> {
    let config: Config = load_config(deps.as_ref())?;

    let balance_request = QueryRequest::Bank(BankQuery::Balance {
        address: owner.to_string(),
        denom: denom.clone(),
    });

    // Query the balance for the specified denom
    let balance_resp: BalanceResponse = deps.querier.query(&balance_request)?;

    Ok(balance_resp.amount.amount)
}

pub fn price_to_tick_index(price: Decimal) -> Result<i64, ContractError> {
    // Ensure the price is greater than 0
    if price.is_zero() || price < Decimal::zero() {
        return Err(ContractError::InvalidPrice);
    }

    // Convert Decimal to f64 by dividing the atomic value by the scaling factor
    let price_f64 = price.atomics().u128() as f64 / 10u128.pow(18) as f64; // 18 is the precision of Decimal

    // Compute the logarithm of the base (1.0001)
    let log_base = 1.0001f64.ln();

    // Compute the logarithm of the price
    let log_price = price_f64.ln();

    // Calculate the tick index using the formula: TickIndex = -log(Price) / log(1.0001)
    let tick_index = -(log_price / log_base);

    // Convert the tick index to i64, rounding to the nearest integer
    Ok(tick_index.round() as i64)
}
