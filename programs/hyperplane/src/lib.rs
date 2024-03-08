#![allow(clippy::result_large_err)]
// #![deny(missing_docs)]

//! An AMM program for the Solana blockchain.

pub mod constraints;
pub mod curve;
pub mod error;
pub mod event;
pub mod instructions;
pub mod ix;
pub mod state;
pub mod utils;

// Export current sdk types for downstream users building with a different sdk version
pub use anchor_lang;
use anchor_lang::prelude::*;
use curve::fees::Fees;
pub use instructions::*;

declare_id!("RaRe29KoKtQcZLoJfeZzzke1KnuVgFV6Lt6Hnjw2QTh");

#[program]
pub mod hyperplane {
    use super::*;
    use crate::event;

    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        curve_parameters: CurveUserParameters,
        fees: Fees,
        initial_supply_a: u64,
        initial_supply_b: u64,
    ) -> Result<()> {
        instructions::initialize_pool::handler_initialize_pool(
            ctx,
            curve_parameters,
            fees,
            initialize_pool::InitialSupply::new(initial_supply_a, initial_supply_b),
        )
    }

    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        minimum_amount_out: u64,
    ) -> Result<event::Swap> {
        instructions::swap::handler_swap(ctx, amount_in, minimum_amount_out)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        pool_token_amount: u64,
        maximum_token_a_amount: u64,
        maximum_token_b_amount: u64,
    ) -> Result<event::Deposit> {
        instructions::deposit::handler_deposit(
            ctx,
            pool_token_amount,
            maximum_token_a_amount,
            maximum_token_b_amount,
        )
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        pool_token_amount: u64,
        minimum_token_a_amount: u64,
        minimum_token_b_amount: u64,
    ) -> Result<event::Withdraw> {
        instructions::withdraw::handler_withdraw(
            ctx,
            pool_token_amount,
            minimum_token_a_amount,
            minimum_token_b_amount,
        )
    }

    pub fn withdraw_fees(
        ctx: Context<WithdrawFees>,
        requested_pool_token_amount: u64,
    ) -> Result<event::WithdrawFees> {
        instructions::withdraw_fees::handler_withdraw_fees(ctx, requested_pool_token_amount)
    }

    pub fn update_pool_config(
        ctx: Context<UpdatePoolConfig>,
        mode: u16,
        value: [u8; VALUE_BYTE_ARRAY_LEN],
    ) -> Result<event::UpdatePoolConfig> {
        instructions::update_pool_config::handler_update_pool_config(ctx, mode, &value)
    }
}
