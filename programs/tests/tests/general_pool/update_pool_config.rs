use crate::utils::*;
use everlend_general_pool::state::{AccountType, SetPoolConfigParams};
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::TransactionError;

async fn setup() -> (ProgramTestContext, TestGeneralPoolMarket, TestGeneralPool) {
    let mut env = presetup().await;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

    let test_pool = TestGeneralPool::new(&test_pool_market, None);
    test_pool
        .create(&mut env.context, &test_pool_market)
        .await
        .unwrap();

    let pool = test_pool.get_data(&mut env.context).await;

    assert_eq!(pool.account_type, AccountType::Pool);

    (env.context, test_pool_market, test_pool)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market, test_pool) = setup().await;

    test_pool
        .set_pool_config(
            &mut context,
            &test_pool_market,
            SetPoolConfigParams {
                deposit_minimum: Some(100),
                withdraw_minimum: Some(150),
            },
        )
        .await
        .unwrap();

    let pool_config = test_pool.get_pool_config(&mut context).await;

    assert_eq!(pool_config.account_type, AccountType::PoolConfig);
    assert_eq!(pool_config.deposit_minimum, 100);
    assert_eq!(pool_config.withdraw_minimum, 150);

    test_pool
        .set_pool_config(
            &mut context,
            &test_pool_market,
            SetPoolConfigParams {
                deposit_minimum: Some(500),
                withdraw_minimum: None,
            },
        )
        .await
        .unwrap();

    let pool_config = test_pool.get_pool_config(&mut context).await;

    assert_eq!(pool_config.deposit_minimum, 500);
    assert_eq!(pool_config.withdraw_minimum, 150);
}

#[tokio::test]
async fn fail_with_wrong_manager() {
    let (mut context, test_pool_market, test_pool) = setup().await;

    let wrong_pool_market = TestGeneralPoolMarket {
        keypair: test_pool_market.keypair,
        manager: Keypair::new(),
    };

    let err = test_pool
        .set_pool_config(
            &mut context,
            &wrong_pool_market,
            SetPoolConfigParams {
                deposit_minimum: Some(100),
                withdraw_minimum: Some(150),
            },
        )
        .await
        .unwrap_err();

    assert_eq!(
        err.unwrap(),
        TransactionError::InstructionError(0, InstructionError::InvalidArgument)
    );
}
