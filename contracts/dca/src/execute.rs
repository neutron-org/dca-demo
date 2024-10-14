use crate::error::ContractError;
use crate::state::{Schedule, CONFIG, SCHEDULES};
use crate::utils::*;
use cosmwasm_std::{BankMsg, Coin, Decimal, CosmosMsg, DepsMut, Env, Int128, MessageInfo, Response, Uint128};
use neutron_std::types::neutron::dex::{LimitOrderType, MsgPlaceLimitOrder};
use neutron_std::types::neutron::dex::DexQuerier;

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

pub fn run_schedule(
    deps: DepsMut,
    env: Env,
) -> Result<Response, ContractError> {
    
    let config = CONFIG.load(deps.storage)?;
    let mut schedules = SCHEDULES.load(deps.storage)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    let mut schedules_to_remove: Vec<u128> = vec![];
    let querier = DexQuerier::new(&deps.querier);

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
        // TODO check price for directionality when neutron_std is fixed
        let basis_point_adjustement = price * Decimal::from_ratio(schedule.max_slippage_basis_points, 10000 as u128);
        //increase target sell price by the basis point adjustement
        let schedule_price = price + basis_point_adjustement;

        // TODO: Fix when neutron_std is updated. We normally only need the price for limit orders, but Neutron_STD doesn't work with 
        // nullible feilds properly so we must also provide the tick index.
        let tick_index_in_to_out = price_to_tick_index(schedule_price)?;

        let estimated_input_amount: Uint128 = match querier.estimate_place_limit_order(
            env.contract.address.to_string(),
            schedule.owner.to_string(),
            token_in.clone(),
            token_out.clone(),
            tick_index_in_to_out,
            sell_amount.to_string(),
            LimitOrderType::ImmediateOrCancel.into(),
            None,
            Int128::MAX.to_string()) {
                Ok(amount) => amount.swap_in_coin.unwrap().amount.parse::<Uint128>().unwrap(),
                Err(_) => Uint128::zero(),
            };

        // if the estimated amount swapped is zero it means that there was no usable liquidity at the price, so we don't include the message
        if estimated_input_amount == Uint128::zero() {
            continue;
        }

        // update the schedule's remaining amount
        if estimated_input_amount < current_schedule_balance {
            schedule.remaining_amount -= estimated_input_amount;
        } else if estimated_input_amount == current_schedule_balance {
            schedule.remaining_amount = Uint128::zero();
            //remove the schedule from the list
            schedules_to_remove.push(schedule.id);
        }
        else{
            //if somehow the amount sold is greater than the user's balance, stop all other executions and error
            return Err(ContractError::InsufficientLiquidity {
                requested: current_schedule_balance,
                available: sell_amount,
            });
        }

        // place an IMMEDIATE_OR_CANCEL limit order. This will sell as much as it can at the price
        // if the price changes before the order is filled the order will be cancelled
        let msg_place_limit_order = Into::<CosmosMsg>::into(MsgPlaceLimitOrder {
            creator: env.contract.address.to_string(),
            receiver: schedule.owner.to_string(),
            token_in: token_in.clone(),
            token_out: token_out.clone(),
            // TODO: Fix when neutron_std is fixed
            tick_index_in_to_out: tick_index_in_to_out,
            amount_in: sell_amount.to_string(),
            order_type: LimitOrderType::ImmediateOrCancel.into(),
            expiration_time: None,
            // TODO: Fix when neutron_std is fixed
            max_amount_out: Int128::MAX.to_string(),
            limit_sell_price: schedule_price.to_string(),
        });

        // Create the response with the deposit message
        messages.push(msg_place_limit_order);
    }
    // Remove marked schedules
    schedules
        .schedules
        .retain(|s| !schedules_to_remove.contains(&s.id));

    // Save the updated config, Config not modified
    SCHEDULES.save(deps.storage, &schedules)?;
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "dex_deposit"))
}

pub fn withdraw_all(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Load the contract configuration to access the owner address and balances
    let config = CONFIG.load(deps.storage)?;
    let mut schedules = SCHEDULES.load(deps.storage)?;
    let mut schedules_to_remove: Vec<u128> = vec![];
    let mut amount_owed: Uint128 = Uint128::zero();
    // loop over schedules and check if any are owned by the user
    for schedule in schedules.schedules.iter() {
        if schedule.owner == info.sender {
            schedules_to_remove.push(schedule.id);
            amount_owed += schedule.remaining_amount;
        }
    }
    // remove the schedules from the list
    schedules
        .schedules
        .retain(|s| !schedules_to_remove.contains(&s.id));
    let mut messages: Vec<CosmosMsg> = vec![];
    // save the schedules
    SCHEDULES.save(deps.storage, &schedules)?;
    // send the funds to the owner
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
    // Return a successful response with the messages to transfer the funds
    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "withdraw")
        .add_attribute("beneficiary", info.sender.to_string())
        .add_attribute("amount", amount_owed.to_string()))
}
