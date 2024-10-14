use crate::error::{ContractError, ContractResult};
use crate::execute::*;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::query::*;
use crate::state::{Config, PairData, CONFIG, SCHEDULES, Schedules };
use crate::utils::*;
use cosmwasm_std::{
    attr, entry_point, Binary, Deps, DepsMut, Env, MessageInfo,
    Response,
};
use cw2::set_contract_version;
use neutron_std::types::slinky::types::v1::CurrencyPair;


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
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
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

    validate_market(&deps_readonly, &env, &currency_pair, msg.max_block_old)?;

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
        max_schedules: msg.max_schedules,
    };
    CONFIG.save(deps.storage, &config)?;

    let schedules = Schedules {
        schedules: Vec::new(),
        nonce: 0,
    };
    SCHEDULES.save(deps.storage, &schedules)?;

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
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::DepositDca { max_sell_amount, max_slippage_basis_points } => deposit_dca(deps, _env, info, max_sell_amount, max_slippage_basis_points),
        ExecuteMsg::RunSchedule { .. } => run_schedule(deps, _env),
        ExecuteMsg::WithdrawAll { .. } => withdraw_all(deps, _env, info),
    }
}

/////////////
/// QUERY ///
/////////////

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::GetFormated {} => query_recent_valid_prices_formatted(deps, _env),
        QueryMsg::GetSchedules { address } => get_schedules(deps, _env, &address),
    }
}
