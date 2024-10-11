use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, CombinedPriceResponse};
use crate::state::{Balances, Config, PairData, CONFIG, USER_BALANCES};
use crate::utils::*;
use crate::execute::*;
use neutron_sdk::bindings::dex::query::{DexQuery, AllUserDepositsResponse};
use neutron_sdk::proto_types::neutron::dex;
use cosmwasm_std::{Addr, Int64};

use cosmwasm_std::{entry_point,
    attr, to_json_binary, Binary, Deps, DepsMut, Env, Int128, MessageInfo, Response, StdResult,
    Uint64, Coin, Uint128, Decimal
};
use cw2::set_contract_version;

pub type ContractResult<T> = core::result::Result<T, ContractError>;
use neutron_sdk::bindings::marketmap::query::{MarketMapQuery, MarketMapResponse, MarketResponse};
use neutron_sdk::bindings::marketmap::types::MarketMap;
use neutron_sdk::bindings::oracle::query::{
    GetAllCurrencyPairsResponse, GetPriceResponse, GetPricesResponse, OracleQuery,
};
use neutron_sdk::bindings::oracle::types::CurrencyPair;
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};

pub fn query_recent_valid_prices_formatted(
    deps: Deps<NeutronQuery>,
    env: Env,
) -> ContractResult<Binary> {
    let price: Decimal = get_price(deps, env)?;

    return Ok(to_json_binary(&price)?);
}

pub fn get_balances(deps: Deps<NeutronQuery>, _env: Env, sender: &Addr) -> ContractResult<Binary> {
    let mut user_balances = USER_BALANCES.may_load(deps.storage, &sender)?
    .unwrap_or_default();  // Use default if not found    let ntrn_balance = user_balances.ntrn;

    Ok(to_json_binary(&user_balances)?)
}

