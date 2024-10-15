use crate::error::ContractResult;
use crate::state::SCHEDULES;
use crate::utils::*;
use cosmwasm_std::{to_json_binary, Addr, Binary, Decimal, Deps, Env};

pub fn query_recent_valid_prices_formatted(deps: Deps, env: Env) -> ContractResult<Binary> {
    let price: Decimal = get_price(deps, env)?;

    return Ok(to_json_binary(&price)?);
}

pub fn get_schedules(deps: Deps, _env: Env, sender: &Addr) -> ContractResult<Binary> {
    let schedules = SCHEDULES.load(deps.storage)?;
    let mut user_schedules = Vec::new();
    for schedule in schedules.schedules {
        if schedule.owner == sender {
            user_schedules.push(schedule);
        }
    }

    Ok(to_json_binary(&user_schedules)?)
}
