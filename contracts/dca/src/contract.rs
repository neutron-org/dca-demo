use crate::error::ContractError;
use crate::execute::*;
use crate::msg::{CombinedPriceResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::query::*;
use crate::state::{Balances, Config, PairData, CONFIG, Schedule};
use crate::utils::*;

use cosmwasm_std::{
    attr, entry_point, to_json_binary, Binary, Coin, Deps, DepsMut, Env, Int128, MessageInfo,
    Response, StdResult, Uint128, Uint64,
};
use cw2::set_contract_version;

pub type ContractResult<T> = core::result::Result<T, ContractError>;
use neutron_sdk::bindings::marketmap::query::{MarketMapQuery, MarketMapResponse, MarketResponse};
use neutron_sdk::bindings::marketmap::types::MarketMap;
use neutron_sdk::bindings::oracle::query::{
    GetAllCurrencyPairsResponse, GetPriceResponse, GetPricesResponse, OracleQuery,
};
use neutron_std::types::slinky::types::v1::CurrencyPair;
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};
///////////////
/// MIGRATE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    unimplemented!()
}

const CONTRACT_NAME: &str = concat!("crates.io:neutron-contracts__", env!("CARGO_PKG_NAME"));
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

///////////////////
/// INSTANTIATE ///
///////////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    msg.validate()?;
    let owner = deps.api.addr_validate(&msg.owner)?;
    let denom_ntrn = msg.denom_ntrn.clone();
    let denom_usd = msg.denom_usd.clone();
    let id = get_pair_id_str(&denom_ntrn, &denom_usd);
    let deps_readonly = Deps {
        storage: deps.storage,
        api: deps.api,
        querier: deps.querier,
    };

    let currency_pair = CurrencyPair {
        base: "NTRN".to_string(),
        quote: "USD".to_string(),
    };

    validate_market(&deps_readonly, &_env, &currency_pair, msg.max_block_old)?;

    let pairs = PairData {
        denom_ntrn: denom_ntrn.clone(),
        denom_usd: denom_usd.clone(),
        currency_pair,
        pair_id: id.clone(),
    };

    let config = Config {
        pair_data: pairs.clone(),
        max_blocks_old: msg.max_block_old,
        owner: owner.clone(),
        schedules: Vec::new(),
        max_schedules: msg.max_schedules,
    };

    // PAIRDATA.save(deps.storage, &pool_data)?;
    CONFIG.save(deps.storage, &config)?;
    
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attributes([
            attr("owner", config.owner.to_string()),
            attr("max_blocks_stale", config.max_blocks_old.to_string()),
            attr("denom_ntrn", pairs.denom_ntrn),
            attr("denom_usdc", pairs.pair_id),
            attr("pool_id", id.clone()),
        ]))
}

///////////////
/// EXECUTE ///
///////////////

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<NeutronMsg>, ContractError> {
    match msg {
        ExecuteMsg::Deposit_dca { .. } => deposit_dca(deps, _env, info),
        ExecuteMsg::Withdraw_dca { .. } => withdraw(deps, _env, info),
        ExecuteMsg::Run_schedule { .. } => deposit_dca(deps, _env, info),
    }
}

/////////////
/// QUERY ///
/////////////

#[entry_point]
pub fn query(deps: Deps<NeutronQuery>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::GetFormated {} => query_recent_valid_prices_formatted(deps, _env),
        QueryMsg::getBalances { address } => get_balances(deps, _env, &address),
    }
}
