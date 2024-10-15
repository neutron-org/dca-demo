use crate::error::ContractError;
use crate::state::{Schedule, CONFIG, SCHEDULES};
use crate::utils::*;
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, Response, SubMsg, SubMsgResult,
    Uint128,
};
use neutron_std::types::neutron::dex::{LimitOrderType, MsgPlaceLimitOrder};

// Deposits a DCA schedule. Users can deposit multiple times to create multiple schedules
// but there is a limit to the total number of schedules
// Only allows DCA buys from USD to NTRN, so only USD_denom can be deposited
pub fn deposit_dca(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    max_sell_amount: Uint128,
    max_slippage_basis_points: u128,
) -> Result<Response, ContractError> {
    // Load the contract configuration from storage
    let config = CONFIG.load(deps.storage)?;
    // get the current user's schedule, default if no schedule found
    let mut schedules = SCHEDULES.load(deps.storage)?;

    // Extract the sent funds from the transaction info
    let sent_funds = info.funds;

    // If no funds are sent, return an error
    if sent_funds.is_empty() {
        return Err(ContractError::NoFundsSent {});
    }
    // Only allow 1 token to be sent
    if sent_funds.len() > 1 {
        return Err(ContractError::MultipleFundsSent {});
    }

    // Check if the schedule count is already maxed out
    if schedules.schedules.len() >= config.max_schedules as usize {
        return Err(ContractError::MaxSchedulesReached {});
    }

    // only allow usd to be sent. since we only allow DCA buys from USD to NTRN
    if sent_funds[0].denom != config.pair_data.denom_usd {
        return Err(ContractError::InvalidToken);
    }

    // Create a new schedule for the user
    let new_schedule = Schedule {
        owner: info.sender.clone(),
        max_sell_amount: max_sell_amount,
        max_slippage_basis_points: max_slippage_basis_points,
        remaining_amount: sent_funds[0].amount,
        id: schedules.nonce,
    };
    schedules.schedules.push(new_schedule);
    schedules.nonce += 1;

    // Save schedules, config not modified
    SCHEDULES.save(deps.storage, &schedules)?;
    // Return a success response with updated balances
    Ok(Response::new()
        .add_attribute("action", "deposit")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("amount", sent_funds[0].amount.to_string()))
}

pub fn run_schedule(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut schedules = SCHEDULES.load(deps.storage)?;

    let mut messages: Vec<SubMsg> = vec![];
    let mut schedules_to_remove: Vec<u128> = vec![];

    // get the current slinky price and tick index.
    let price = get_price(deps.as_ref(), env.clone())?;

    // Loop over all schedules
    for schedule in schedules.schedules.iter_mut() {
        let current_schedule_balance: Uint128 = schedule.remaining_amount;
        // Check if the current schedule balance is 0
        if current_schedule_balance.is_zero() {
            // Mark this schedule for removal
            schedules_to_remove.push(schedule.id);
            continue;
        }

        // sell amount is the min of the current schedule balance and the max_sell_amount
        let sell_amount = std::cmp::min(current_schedule_balance, schedule.max_sell_amount);
        let (token_in, token_out) = (
            config.pair_data.denom_usd.clone(),
            config.pair_data.denom_ntrn.clone(),
        );

        // the schedule_price is the price with the slippage_adjustment applied
        let basis_point_adjustement =
            price + Decimal::from_ratio(schedule.max_slippage_basis_points, 10000u128);
        //increase target sell price by the basis point adjustement
        let schedule_price = price + basis_point_adjustement;
        let target_tick_index = price_to_tick_index(schedule_price);

        // place an IMMEDIATE_OR_CANCEL limit order. This will sell as much as it can at the price
        // if the price changes before the order is filled the order will be cancelled
        let msg_place_limit_order = Into::<CosmosMsg>::into(MsgPlaceLimitOrder {
            creator: env.contract.address.to_string(),
            receiver: schedule.owner.to_string(),
            token_in: token_in.clone(),
            token_out: token_out.clone(),
            tick_index_in_to_out: target_tick_index.unwrap(),
            amount_in: sell_amount.to_string(),
            order_type: LimitOrderType::ImmediateOrCancel.into(),
            expiration_time: None,
            min_average_sell_price: None,
            max_amount_out: None,
            limit_sell_price: None,
        });

        // push SubMsg
        messages.push(SubMsg::reply_on_success(
            msg_place_limit_order,
            schedule.id as u64,
        ));
    }
    // Remove marked schedules
    schedules
        .schedules
        .retain(|s| !schedules_to_remove.contains(&s.id));

    // Save the updated config, Config not modified
    SCHEDULES.save(deps.storage, &schedules)?;
    Ok(Response::new()
        .add_submessages(messages)
        .add_attribute("action", "dex_deposit"))
}

pub fn withdraw_all(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut schedules = SCHEDULES.load(deps.storage)?;
    let mut amount_owed: Uint128 = Uint128::zero();

    // Use retain to remove schedules and calculate amount owed in one pass
    schedules.schedules.retain(|schedule| {
        if schedule.owner == info.sender {
            amount_owed += schedule.remaining_amount;
            false // Remove this schedule
        } else {
            true // Keep this schedule
        }
    });

    // Save the updated schedules
    SCHEDULES.save(deps.storage, &schedules)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    if !amount_owed.is_zero() {
        messages.push(
            BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    denom: config.pair_data.denom_usd.clone(),
                    amount: amount_owed,
                }],
            }
            .into(),
        );
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "withdraw")
        .add_attribute("beneficiary", info.sender.to_string())
        .add_attribute("amount", amount_owed.to_string()))
}

pub fn handle_run_schedule_reply(
    deps: DepsMut,
    _env: Env,
    msg_result: SubMsgResult,
    schedule_id: u64,
) -> Result<Response, ContractError> {
    match msg_result {
        SubMsgResult::Ok(result) => {
            let amount_in = extract_amount_in(&result)?;
            let mut schedules = SCHEDULES.load(deps.storage)?;

            update_schedules(&mut schedules, schedule_id, amount_in)?;

            SCHEDULES.save(deps.storage, &schedules)?;

            Ok(Response::new()
                .add_attribute("action", "place_limit_order_reply_success")
                .add_attribute("schedule_id", schedule_id.to_string())
                .add_attribute("amount_in", amount_in.to_string()))
        }
        SubMsgResult::Err(err) => Ok(Response::new()
            .add_attribute("action", "place_limit_order_reply_error")
            .add_attribute("error", err)
            .add_attribute("schedule_id", schedule_id.to_string())),
    }
}
