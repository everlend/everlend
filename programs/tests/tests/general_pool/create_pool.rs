use crate::utils::*;
use everlend_general_pool::instruction;
use everlend_general_pool::state::AccountType;
use solana_program::instruction::InstructionError;
use solana_program::system_instruction::SystemError;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

async fn setup() -> (ProgramTestContext, TestGeneralPoolMarket) {
    let mut env = presetup().await;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market.init(&mut env.context, &env.registry.keypair.pubkey()).await.unwrap();

    (env.context, test_pool_market)
}

#[tokio::test]
async fn success() {
    let (mut context, test_pool_market) = setup().await;

    let test_pool = TestGeneralPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let pool = test_pool.get_data(&mut context).await;

    assert_eq!(pool.account_type, AccountType::Pool);

    let withdrawal_requests = test_pool
        .get_withdrawal_requests(&mut context, &test_pool_market)
        .await
        .1;

    assert_eq!(
        withdrawal_requests.account_type,
        AccountType::WithdrawRequests
    );
    assert_eq!(withdrawal_requests.pool, test_pool.pool_pubkey,);
}

#[tokio::test]
async fn fail_second_time_init() {
    let (mut context, test_pool_market) = setup().await;

    let test_pool = TestGeneralPool::new(&test_pool_market, None);
    test_pool
        .create(&mut context, &test_pool_market)
        .await
        .unwrap();

    let pool = test_pool.get_data(&mut context).await;

    assert_eq!(pool.account_type, AccountType::Pool);

    context.warp_to_slot(3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::create_pool(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool.token_mint_pubkey,
            &test_pool.token_account.pubkey(),
            &test_pool.pool_mint.pubkey(),
            &test_pool_market.manager.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager],
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
            InstructionError::Custom(SystemError::AccountAlreadyInUse as u32)
        )
    );
}
