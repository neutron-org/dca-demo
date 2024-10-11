use cosmwasm_std::{Addr, Int64};
use cw_storage_plus::{Item, Map};
use neutron_std::types::slinky::types::v1::CurrencyPair;
use crate::{
    error::{ContractError, ContractResult},
};

// use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Response, Uint128};
use neutron_sdk::bindings::marketmap::query::{MarketMapQuery, MarketMapResponse, MarketResponse};
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};


use neutron_sdk::bindings::oracle::query::{
    GetAllCurrencyPairsResponse, GetPriceResponse, GetPricesResponse, OracleQuery,
};
use cosmwasm_std::Uint64;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct PairData {
    pub denom_ntrn: String,
    pub denom_usd: String,
    pub currency_pair: CurrencyPair,
    pub pair_id: String,
}

/// This structure stores the concentrated pair parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Balances {
    pub ntrn: Coin,
    pub usd: Coin
}

impl Default for Balances {
    fn default() -> Self {
        Self {
            ntrn: Coin { denom: "untrn".to_string(), amount: Uint128::zero() },
            usd: Coin { denom: "uusd".to_string(), amount: Uint128::zero() },
        }
    }
}
/// This structure stores the concentrated pair parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Schedule {
    pub owner: Addr,
    pub denom: String,
    pub max_sell_amount: Uint128,
    pub max_slippage_basis_points: u16,
}

/// This structure stores the concentrated pair parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    /// number of blocks until price is stale
    pub pair_data: PairData,
    pub max_blocks_old: u64,
    pub owner: Addr,
    pub schedules: Vec<Schedule>,
    pub max_schedules: u64,
}

// pub const PAIRDATA: Item<PairData> = Item::new("data");
pub const CONFIG: Item<Config> = Item::new("data");
pub const USER_BALANCES: Map<&Addr, Balances> = Map::new("user_balances");