#![cfg(feature = "test-bpf")]

use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use everlend_registry::state::{AccountType, SetRegistryConfigParams, TOTAL_DISTRIBUTIONS};
use everlend_utils::EverlendError;

use crate::utils::*;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let mut config = SetRegistryConfigParams {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        refresh_income_interval: REFRESH_INCOME_INTERVAL,
    };
    config.money_market_program_ids[0] = spl_token_lending::id();

    test_registry
        .set_registry_config(&mut context, config)
        .await
        .unwrap();

    let registry_config = test_registry.get_config_data(&mut context).await;
    assert_eq!(registry_config.account_type, AccountType::RegistryConfig);
}

#[tokio::test]
async fn success_change_registry_config() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let config = SetRegistryConfigParams {
        general_pool_program_id: Pubkey::new_unique(),
        ulp_program_id: Pubkey::new_unique(),
        liquidity_oracle_program_id: Pubkey::new_unique(),
        depositor_program_id: Pubkey::new_unique(),
        income_pools_program_id: Pubkey::new_unique(),
        money_market_program_ids: [Pubkey::new_unique(); TOTAL_DISTRIBUTIONS],
        refresh_income_interval: REFRESH_INCOME_INTERVAL,
    };

    test_registry
        .set_registry_config(&mut context, config)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    let registry_config = test_registry.get_config_data(&mut context).await;
    assert_eq!(registry_config.account_type, AccountType::RegistryConfig);

    let config = SetRegistryConfigParams {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [spl_token_lending::id(); TOTAL_DISTRIBUTIONS],
        refresh_income_interval: REFRESH_INCOME_INTERVAL,
    };

    test_registry
        .set_registry_config(&mut context, config)
        .await
        .unwrap();

    let registry_config = test_registry.get_config_data(&mut context).await;

    assert_eq!(
        SetRegistryConfigParams {
            general_pool_program_id: registry_config.general_pool_program_id,
            ulp_program_id: registry_config.ulp_program_id,
            liquidity_oracle_program_id: registry_config.liquidity_oracle_program_id,
            depositor_program_id: registry_config.depositor_program_id,
            income_pools_program_id: registry_config.income_pools_program_id,
            money_market_program_ids: registry_config.money_market_program_ids,
            refresh_income_interval: registry_config.refresh_income_interval,
        },
        config
    )
}

#[tokio::test]
async fn fail_with_invalid_registry() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let mut config = SetRegistryConfigParams {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        refresh_income_interval: REFRESH_INCOME_INTERVAL,
    };
    config.money_market_program_ids[0] = spl_token_lending::id();

    let tx = Transaction::new_signed_with_payer(
        &[everlend_registry::instruction::set_registry_config(
            &everlend_registry::id(),
            &Pubkey::new_unique(),
            &test_registry.manager.pubkey(),
            config,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_registry.manager],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_wrong_manager() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let mut config = SetRegistryConfigParams {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        refresh_income_interval: REFRESH_INCOME_INTERVAL,
    };
    config.money_market_program_ids[0] = spl_token_lending::id();

    let wrong_manager = Keypair::new();

    let tx = Transaction::new_signed_with_payer(
        &[everlend_registry::instruction::set_registry_config(
            &everlend_registry::id(),
            &test_registry.keypair.pubkey(),
            &wrong_manager.pubkey(),
            config,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &wrong_manager],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );
}
