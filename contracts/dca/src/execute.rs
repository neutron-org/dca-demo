use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Balances, Config, PairData, Schedule, CONFIG, USER_BALANCES};
use crate::utils::*;

use cosmwasm_std::{
    attr, entry_point, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    Int128, MessageInfo, QueryRequest, Response, StdResult, Uint128, Uint64,
};
use cw2::set_contract_version;

pub type ContractResult<T> = core::result::Result<T, ContractError>;

use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};
use serde_json::to_string;
use neutron_std::types::neutron::dex::MsgPlaceLimitOrder;
use neutron_std::types::neutron::dex::LimitOrderType;


pub fn deposit_dca(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
) -> Result<Response<NeutronMsg>, ContractError> {
    // Load the contract configuration from storage
    let mut config = CONFIG.load(deps.storage)?;
    let mut user_balances = USER_BALANCES.may_load(deps.storage, &info.sender)?
    .unwrap_or_default();  // Use default if not found
    // Extract the sent funds from the transaction info
    let sent_funds = info.funds;
    
    // If no funds are sent, return an error
    if sent_funds.is_empty() {
        return Err(ContractError::NoFundsSent {});
    }

    if sent_funds.len() > 1 {
        return Err(ContractError::MultipleFundsSent{});
    }
    
    // Check if the user already has a balance
    if !user_balances.ntrn.amount.is_zero() || !user_balances.usd.amount.is_zero() {
        return Err(ContractError::ExistingBalance {});
    }

    // Check if the schedule is already maxed out
    if config.schedules.len() >= config.max_schedules as usize {
        return Err(ContractError::MaxSchedulesReached {});
    }

    // Create a new schedule for the user
    let new_schedule = Schedule {
        owner: info.sender.clone(),
        max_sell_amount: sent_funds[0].amount,
        max_slippage_basis_points: 10,
        denom: sent_funds[0].denom.clone(),
    };

    // Add the new schedule to the config
    config.schedules.push(new_schedule);

    // Iterate through the sent funds and update the contract's balances
    for coin in sent_funds.iter() {
        if coin.denom == user_balances.ntrn.denom {
            user_balances.ntrn.amount += coin.amount;
        } else if coin.denom == user_balances.usd.denom {
            user_balances.usd.amount += coin.amount;
        } else {
            // Return an error if an unsupported token is sent
            return Err(ContractError::InvalidToken);
        }
    }

    // Save the updated configuration with new balances back to the contract's storage
    USER_BALANCES.save(deps.storage, &info.sender, &user_balances)?;
    CONFIG.save(deps.storage, &config)?;
    // Return a success response with updated balances
    Ok(Response::new()
        .add_attribute("action", "deposit")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("token_0_amount", user_balances.ntrn.amount.to_string())
        .add_attribute("token_1_amount", user_balances.usd.amount.to_string()))
}


pub fn run_schedule(
    deps: DepsMut<NeutronQuery>,
    env: Env,
    info: MessageInfo,
) -> Result<Response<NeutronMsg>, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let mut user_balances = USER_BALANCES.load(deps.storage, &info.sender)?;

    let mut messages: Vec<NeutronMsg> = vec![];

    // get the current slinky price and tick index
    let price = get_price(deps.as_ref(), env.clone())?;
    let tick_index = price_to_tick_index(price)?;

    // Loop over all schedules
    for schedule in config.schedules.iter_mut() {
        let current_schedule_balance: Uint128 = get_user_balance(&deps, schedule.denom, schedule.owner)?;

        // Check if the current schedule balance is 0
        if current_schedule_balance.is_zero() {
            // Update user balances
            if schedule.denom == user_balances.ntrn.denom {
                user_balances.ntrn.amount = Uint128::zero();
            } else if schedule.denom == user_balances.usd.denom {
                user_balances.usd.amount = Uint128::zero();
            }
            
            // Remove this schedule from the config
            config.schedules.retain(|s| s.owner != schedule.owner || s.denom != schedule.denom);
    
            // Save the updated config and user balances
            CONFIG.save(deps.storage, &config)?;
            USER_BALANCES.save(deps.storage, &info.sender, &user_balances)?;

            // Exit the function with a response
            return Ok(Response::new()
                .add_attribute("action", "run_schedule")
                .add_attribute("result", "schedule_removed")
                .add_attribute("reason", "zero_balance"));
        }

        let sell_amount = std::cmp::min(current_schedule_balance, schedule.max_sell_amount);
       
        let (token_in, token_out) = if schedule.denom == config.pair_data.denom_ntrn {
            (config.pair_data.denom_ntrn.clone(), config.pair_data.denom_usd.clone())
        } else {
            (config.pair_data.denom_usd.clone(), config.pair_data.denom_ntrn.clone())
        };

        let msg_place_limit_order = MsgPlaceLimitOrder {
            creator: env.contract.address.to_string(),
            receiver: schedule.owner.to_string(),
            token_in: token_in.clone(),
            token_out: token_out.clone(),
            tick_index_in_to_out: tick_index,
            amount_in: sell_amount.to_string(),
            order_type: LimitOrderType::FillOrKill.into(),
            expiration_time: None,
            max_amount_out: Int128::MAX.to_string(),
            limit_sell_price: price.to_string(),
        };
         // Create the response with the deposit message
        messages.push(msg_place_limit_order);
    }


    Ok(Response::<CosmosMsg>::new()
        .add_messages(messages)
        .add_attribute("action", "dex_deposit"))
}

pub fn dex_withdrawal(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
) -> Result<Response<NeutronMsg>, ContractError> {
    unimplemented!()
}


pub fn withdraw(
    deps: DepsMut<NeutronQuery>,
    _env: Env,
    info: MessageInfo,
) -> Result<Response<NeutronMsg>, ContractError> {
    // Load the contract configuration to access the owner address and balances
    let mut config = CONFIG.load(deps.storage)?;
    let mut user_balances = USER_BALANCES.load(deps.storage, &info.sender)?;

    // Verify that the sender is the owner
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    // Check if there are any funds to withdraw
    if user_balances.ntrn.amount.is_zero() && user_balances.usd.amount.is_zero() {
        return Err(ContractError::NoFundsAvailable {});
    }

    // Prepare messages to send the entire balance of each token back to the owner
    let mut messages: Vec<cosmwasm_std::CosmosMsg<NeutronMsg>> = vec![];

    if !user_balances.ntrn.amount.is_zero() {
        messages.push(
            BankMsg::Send {
                to_address: config.owner.to_string(),
                amount: vec![Coin {
                    denom: user_balances.ntrn.denom.clone(),
                    amount: user_balances.ntrn.amount,
                }],
            }
            .into(),
        );
    }

    if !user_balances.usd.amount.is_zero() {
        messages.push(
            BankMsg::Send {
                to_address: config.owner.to_string(),
                amount: vec![Coin {
                    denom: user_balances.usd.denom.clone(),
                    amount: user_balances.usd.amount,
                }],
            }
            .into(),
        );
    }

    // Reset the balances to zero after withdrawal
    user_balances.ntrn.amount = Uint128::zero();
    user_balances.usd.amount = Uint128::zero();

    // Save the updated config (with zeroed balances) back to storage
    CONFIG.save(deps.storage, &config)?;
    USER_BALANCES.save(deps.storage, &info.sender, &user_balances)?;
    // Return a successful response with the messages to transfer the funds
    Ok(Response::<NeutronMsg>::new()
        .add_messages(messages)
        .add_attribute("action", "withdraw_all")
        .add_attribute("owner", config.owner.to_string())
        .add_attribute(
            "token_0_withdrawn",
            user_balances.ntrn.amount.to_string(),
        )
        .add_attribute(
            "token_1_withdrawn",
            user_balances.usd.amount.to_string(),
        ))
}