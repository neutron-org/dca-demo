use crate::{
    error::{ContractError, ContractResult},
    state::PairData,
};
use cosmwasm_std::{Addr, Int64};

// use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Decimal, Response, Uint128};
use neutron_sdk::bindings::marketmap::query::{MarketMapQuery, MarketMapResponse, MarketResponse};
use neutron_sdk::bindings::oracle::types::CurrencyPair;
use neutron_sdk::bindings::{msg::NeutronMsg, query::NeutronQuery};

use cosmwasm_std::Uint64;
use neutron_sdk::bindings::oracle::query::{
    GetAllCurrencyPairsResponse, GetPriceResponse, GetPricesResponse, OracleQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct DepositOptions {
    pub token_a: Option<Coin>,
    pub token_b: Option<Coin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ReceiveFunds {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub owner: String,
    pub denom_ntrn: String,
    pub denom_usd: String,
    pub max_block_old: u64,
    pub max_schedules: u64,
}

impl InstantiateMsg {
    pub fn validate(&self) -> ContractResult<()> {
        self.check_empty(self.owner.clone(), "beneficiary".to_string())?;
        self.check_empty(self.denom_ntrn.clone(), "ntrn".to_string())?;
        self.check_empty(self.denom_usd.clone(), "usd".to_string())?;

        if self.max_block_old <= 0 {
            return Err(ContractError::MalformedInput {
                input: "max_block_stale".to_string(),
                reason: "must be >=1".to_string(),
            });
        }
        Self::validate_denom(&self.denom_ntrn)?;
        Self::validate_denom(&self.denom_usd)?;
        Ok(())
    }
    fn validate_denom(denom: &str) -> ContractResult<Response> {
        let invalid_denom = |reason: &str| {
            Err(ContractError::InvalidIbcDenom {
                denom: String::from(denom),
                reason: reason.to_string(),
            })
        };
        // if it's an IBC denom
        if denom.len() >= 4 && denom.starts_with("ibc/") {
            // Step 1: Validate length
            if denom.len() != 68 {
                return invalid_denom("expected length of 68 chars");
            }

            // Step 2: Validate prefix
            if !denom.starts_with("ibc/") {
                return invalid_denom("expected prefix 'ibc/'");
            }

            // Step 3: Validate hash
            if !denom
                .chars()
                .skip(4)
                // c.is_ascii_hexdigit() could have been used here, but it allows lowercase characters
                .all(|c| matches!(c, '0'..='9' | 'A'..='F'))
            {
                return invalid_denom("invalid denom hash");
            }
        }
        Ok(Response::new())
    }
    fn validate_currency_pair(pair: &CurrencyPair) -> ContractResult<Response> {
        let invalid_pair = |reason: &str| {
            Err(ContractError::InvalidCurrencyPair {
                base: String::from(&pair.base),
                quote: String::from(&pair.quote),
                reason: reason.to_string(),
            })
        };
        // Check if base is NTRN and quote is USD
        if pair.base != "NTRN" || pair.quote != "USD" {
            return invalid_pair("base must be NTRN and quote must be USD");
        }
        Ok(Response::new())
    }
    pub fn check_empty(&self, input: String, kind: String) -> ContractResult<()> {
        if input.is_empty() {
            return Err(ContractError::EmptyValue { kind: kind });
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // deposit funds to be DCA's
    Deposit_dca {max_sell_amount: Uint128, max_slippage_basis_points: u16},
    // withdraws any remaining funds form the DCA strategy
    Withdraw_dca {},
    // withdraws any remaining funds form the DCA strategy
    Run_schedule {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetFormated {},
    getBalances {
        address: Addr
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CombinedPriceResponse {
    pub token_0_price: Decimal,
    pub token_1_price: Decimal,
    pub price_0_to_1: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct DepositResult {
    pub amount0: Uint128,
    pub amount1: Uint128,
    pub tick_index: i64,
    pub fee: u64,
}
