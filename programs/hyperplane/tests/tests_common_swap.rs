// useful for d.p. clarity in tests
#![allow(clippy::inconsistent_digit_grouping)]

mod common;

use common::{client, runner};
use hyperplane::{
    curve::{calculator::TradeDirection, fees::Fees},
    error::SwapError,
    ix::{Swap, UpdatePoolConfig},
    state::{SwapState, UpdatePoolConfigMode, UpdatePoolConfigValue},
    CurveUserParameters, InitialSupply,
};
use solana_program_test::tokio::{self};
use test_case::test_case;

use crate::common::{
    fixtures, setup, setup::default_supply, state, token_operations, types::{SwapPairSpec, TokenSpec},
};

#[tokio::test]
pub async fn test_swap_fails_with_withdrawal_only_mode() {
    let program = runner::program(&[]);
    let mut ctx = runner::start(program).await;

    let pool = fixtures::new_pool(
        &mut ctx,
        Fees::default(),
        default_supply(),
        SwapPairSpec::default(),
        CurveUserParameters::Stable { amp: 100 },
    )
    .await;

    client::update_pool_config(
        &mut ctx,
        &pool,
        UpdatePoolConfig::new(
            UpdatePoolConfigMode::WithdrawalsOnly,
            UpdatePoolConfigValue::Bool(true),
        ),
    )
    .await
    .unwrap();
    let pool_state = state::get_pool(&mut ctx, &pool).await;
    assert!(pool_state.withdrawals_only());

    let user = setup::new_pool_user(&mut ctx, &pool, (50, 0)).await;
    assert_eq!(
        client::swap(
            &mut ctx,
            &pool,
            &user,
            TradeDirection::AtoB,
            Swap {
                amount_in: 50,
                minimum_amount_out: 47,
            },
        )
        .await
        .unwrap_err()
        .unwrap(),
        hyperplane_error!(SwapError::WithdrawalsOnlyMode)
    );

    // unset withdrawals_only mode
    client::update_pool_config(
        &mut ctx,
        &pool,
        UpdatePoolConfig::new(
            UpdatePoolConfigMode::WithdrawalsOnly,
            UpdatePoolConfigValue::Bool(false),
        ),
    )
    .await
    .unwrap();
    client::swap(
        &mut ctx,
        &pool,
        &user,
        TradeDirection::AtoB,
        Swap {
            amount_in: 50,
            minimum_amount_out: 47,
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
pub async fn test_swap_with_host_fees_less_than_one_rounds_down_to_zero() {
    let program = runner::program(&[]);
    let mut ctx = runner::start(program).await;

    let pool = fixtures::new_pool(
        &mut ctx,
        Fees {
            trade_fee_numerator: 1,
            trade_fee_denominator: 100,
            owner_trade_fee_numerator: 1,
            owner_trade_fee_denominator: 100,
            host_fee_numerator: 1,
            host_fee_denominator: 100,
            ..Default::default()
        },
        InitialSupply::new(100, 100),
        SwapPairSpec::default(),
        CurveUserParameters::Stable { amp: 100 },
    )
    .await;

    let user = setup::new_pool_user(&mut ctx, &pool, (50, 0)).await;
    let host_fees = setup::new_pool_user(&mut ctx, &pool, (0, 0)).await;

    client::swap_with_host_fees(
        &mut ctx,
        &pool,
        &user,
        Some(&host_fees),
        TradeDirection::AtoB,
        Swap {
            amount_in: 50,
            minimum_amount_out: 44,
        },
    )
    .await
    .unwrap();

    let vault_a_balance = token_operations::balance(&mut ctx, &pool.token_a_vault).await;
    assert_eq!(vault_a_balance, 149);
    let vault_b_balance = token_operations::balance(&mut ctx, &pool.token_b_vault).await;
    assert_eq!(vault_b_balance, 53);

    // owner get the 1 fee payed into fee vault
    let token_a_fees_vault_balance =
        token_operations::balance(&mut ctx, &pool.token_a_fees_vault).await;
    assert_eq!(token_a_fees_vault_balance, 1);
    let token_b_fees_vault_balance =
        token_operations::balance(&mut ctx, &pool.token_b_fees_vault).await;
    assert_eq!(token_b_fees_vault_balance, 0);

    // no host fees payed host fee account
    let host_token_a_fees_balance =
        token_operations::balance(&mut ctx, &host_fees.token_a_ata).await;
    assert_eq!(host_token_a_fees_balance, 0);

    let user_a_balance = token_operations::balance(&mut ctx, &user.token_a_ata).await;
    let user_b_balance = token_operations::balance(&mut ctx, &user.token_b_ata).await;
    assert_eq!(user_a_balance, 0);
    assert_eq!(user_b_balance, 47);
}

#[tokio::test]
pub async fn test_swap_trade_and_owner_fee_correct_ratio() {
    let program = runner::program(&[]);
    let mut ctx = runner::start(program).await;

    let pool = fixtures::new_pool(
        &mut ctx,
        Fees {
            trade_fee_numerator: 1000,
            trade_fee_denominator: 10000,
            owner_trade_fee_numerator: 1000,
            owner_trade_fee_denominator: 10000,
            host_fee_numerator: 0,
            host_fee_denominator: 10000,
            ..Default::default()
        },
        InitialSupply::new(10000, 10000),
        SwapPairSpec{ a: TokenSpec::transfer_fees(1000), b: TokenSpec::spl_token(6) },
        CurveUserParameters::ConstantPrice { token_b_price: 1 },
    )
    .await;

    let user = setup::new_pool_user(&mut ctx, &pool, (10000, 0)).await;

    client::swap_with_host_fees(
        &mut ctx,
        &pool,
        &user,
        None,
        TradeDirection::AtoB,
        Swap {
            amount_in: 10000,
            minimum_amount_out: 0,
        },
    )
    .await
    .unwrap();
    let user_a_balance = token_operations::balance(&mut ctx, &user.token_a_ata).await;
    let user_b_balance = token_operations::balance(&mut ctx, &user.token_b_ata).await;
    assert_eq!(user_a_balance, 0);
    assert_eq!(user_b_balance, 7200);
}

#[test_case(   0,    0,    0,    0; "all fees zero")]
#[test_case(1000,    0,    0,    0; "only pool fee")]
#[test_case(   0, 1000,    0,    0; "only owner fee")]
#[test_case(   0, 1000, 1000,    0; "owner fee + host fee")]
#[test_case(1000, 1000,    0,    0; "pool fee + owner fee")]
#[test_case(1000, 1000, 1000,    0; "pool fee + owner fee + host fee")]
#[test_case(   0,    0,    0, 1000; "token22 fee + all fees zero")]
#[test_case(1000,    0,    0, 1000; "token22 fee + only pool fee")]
#[test_case(   0, 1000,    0, 1000; "token22 fee + only owner fee")]
#[test_case(   0, 1000, 1000, 1000; "token22 fee + owner fee + host fee")]
#[test_case(1000, 1000,    0, 1000; "token22 fee + pool fee + owner fee")]
#[test_case(1000, 1000, 1000, 1000; "token22 fee + pool fee + owner fee + host fee")]
#[tokio::test]
pub async fn test_swap_invariants_correct_trade(trade_fee_numerator: u64, owner_trade_fee_numerator: u64, host_fee_numerator: u64, token_a_transfer_fees: u16) {
    let program = runner::program(&[]);
    let mut ctx = runner::start(program).await;

    let pool = fixtures::new_pool(
        &mut ctx,
        Fees {
            trade_fee_numerator,
            trade_fee_denominator: 10000,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator: 10000,
            host_fee_numerator,
            host_fee_denominator: 10000,
            ..Default::default()
        },
        InitialSupply::new(10000, 10000),
        SwapPairSpec{ a: TokenSpec::transfer_fees(token_a_transfer_fees), b: TokenSpec::spl_token(6) },
        CurveUserParameters::ConstantPrice { token_b_price: 1 },
    )
    .await;

    let user = setup::new_pool_user(&mut ctx, &pool, (10000, 0)).await;
    let host_fees = setup::new_pool_user(&mut ctx, &pool, (0, 0)).await;
    let vault_a_balance_before = token_operations::balance(&mut ctx, &pool.token_a_vault).await;
    let vault_b_balance_before = token_operations::balance(&mut ctx, &pool.token_b_vault).await;

    client::swap_with_host_fees(
        &mut ctx,
        &pool,
        &user,
        Some(&host_fees),
        TradeDirection::AtoB,
        Swap {
            amount_in: 10000,
            minimum_amount_out: 0,
        },
    )
    .await
    .unwrap();
    let user_a_balance = token_operations::balance(&mut ctx, &user.token_a_ata).await;
    // In all of the cases user should consume all of his A
    assert_eq!(user_a_balance, 0);

    let vault_a_balance_after = token_operations::balance(&mut ctx, &pool.token_a_vault).await;
    let vault_b_balance_after = token_operations::balance(&mut ctx, &pool.token_b_vault).await;
    let vault_a_diff = vault_a_balance_after - vault_a_balance_before;
    let vault_b_diff = vault_b_balance_before - vault_b_balance_after;
    // And the tokens A gained by the pool should be higher than tokens B lost (we assume that price of A/B is constant 1)
    assert!(vault_a_diff >= vault_b_diff);
}
