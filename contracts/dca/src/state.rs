use cosmwasm_std::Addr;
use cosmwasm_std::Uint128;
use cw_storage_plus::Item;
use neutron_std::types::slinky::types::v1::CurrencyPair;
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
pub struct Schedule {
    // the remaining amount in the schedule
    pub remaining_amount: Uint128,
    // owner and beneficiary of the schedule
    pub owner: Addr,
    // the max amount of the usd_denom to sell per schedule run
    pub max_sell_amount: Uint128,
    // the max slippage in basis points the schedule owner is willing to accept
    pub max_slippage_basis_points: u128,
    // the unique id of the schedule
    pub id: u128,
}
/// This structure stores the concentrated pair parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Schedules {
    // vec of all active schedules
    pub schedules: Vec<Schedule>,
    // global schedules nonce used to set unique IDs
    pub nonce: u128,
}

/// This structure stores the concentrated pair parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub pair_data: PairData,
    // the max blocks old for the oracle price
    pub max_blocks_old: u64,
    // the owner of the contract
    pub owner: Addr,
    // the max number of schedules
    pub max_schedules: u64,
}

// pub const PAIRDATA: Item<PairData> = Item::new("data");
pub const CONFIG: Item<Config> = Item::new("data");
pub const SCHEDULES: Item<Schedules> = Item::new("user_schedules");
