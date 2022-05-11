#![cfg(feature = "test-bpf")]

use solana_program::instruction::InstructionError;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::signature::Keypair;
use solana_program_test::*;
use everlend_registry::state::SetPoolConfigParams;
use everlend_registry::state::{AccountType};
use solana_sdk::transaction::{Transaction, TransactionError};
use everlend_utils::EverlendError;

use crate::utils::*;

async fn setup() -> (
    ProgramTestContext,
    TestRegistry,
    TestGeneralPool,
) {
    let mut context = program_test().start_with_context().await;
    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut context, &test_registry.keypair.pubkey()).await.unwrap();
    let test_pool = TestGeneralPool::new(&test_pool_market, Some(spl_token::native_mint::id()));
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();
    (context, test_registry, test_pool)
}

#[tokio::test]
async fn success() {
    let (mut context, test_registry, test_pool) = setup().await;
    let pool_config_params = SetPoolConfigParams { deposit_minimum: 100, withdraw_minimum: 100 };
    test_registry
        .set_pool_config(
            &mut context,
            &test_pool.pool_pubkey,
            pool_config_params,
        )
        .await
        .unwrap();

    let pool_config = test_registry.get_pool_config(&mut context, &test_pool.pool_pubkey).await;
    assert_eq!(pool_config.account_type, AccountType::PoolConfig);
    assert_eq!(pool_config.deposit_minimum, pool_config_params.deposit_minimum);
    assert_eq!(pool_config.withdraw_minimum, pool_config_params.withdraw_minimum);
}

#[tokio::test]
async fn success_change_pool_config() {
    let (mut context, test_registry, test_pool) = setup().await;

    let pool_config_params = SetPoolConfigParams { deposit_minimum: 100, withdraw_minimum: 100 };
    test_registry.set_pool_config(&mut context, &test_pool.pool_pubkey, pool_config_params).await.unwrap();
    let pool_config = test_registry.get_pool_config(&mut context, &test_pool.pool_pubkey).await;
    assert_eq!(pool_config.account_type, AccountType::PoolConfig);
    assert_eq!(pool_config.deposit_minimum, pool_config_params.deposit_minimum);
    assert_eq!(pool_config.withdraw_minimum, pool_config_params.withdraw_minimum);

    context.warp_to_slot(3).unwrap();
    let changed_pool_config = SetPoolConfigParams { deposit_minimum: 200, withdraw_minimum: 200 };
    test_registry.set_pool_config(&mut context, &test_pool.pool_pubkey, changed_pool_config).await.unwrap();
    let pool_config = test_registry.get_pool_config(&mut context, &test_pool.pool_pubkey).await;
    assert_eq!(pool_config.deposit_minimum, changed_pool_config.deposit_minimum);
    assert_eq!(pool_config.withdraw_minimum, changed_pool_config.withdraw_minimum);
}

#[tokio::test]
async fn fail_with_invalid_registry() {
    let (mut context, test_registry, test_pool) = setup().await;

    let config = SetPoolConfigParams {
        deposit_minimum: 100,
        withdraw_minimum: 100,
    };

    let tx = Transaction::new_signed_with_payer(
        &[everlend_registry::instruction::set_pool_config(
            &everlend_registry::id(),
            &Pubkey::new_unique(),
            &test_registry.manager.pubkey(),
            &test_pool.pool_pubkey,
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
    let (mut context, test_registry, test_pool) = setup().await;

    let config = SetPoolConfigParams {
        deposit_minimum: 100,
        withdraw_minimum: 100,
    };

    let wrong_manager = Keypair::new();
    let tx = Transaction::new_signed_with_payer(
        &[everlend_registry::instruction::set_pool_config(
            &everlend_registry::id(),
            &test_registry.keypair.pubkey(),
            &wrong_manager.pubkey(),
            &test_pool.pool_pubkey,
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
