use crate::utils::*;
use everlend_general_pool::instruction;
use everlend_general_pool::state::PoolMarket;
use solana_program::instruction::InstructionError;
use solana_program::program_pack::Pack;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

async fn setup() -> (ProgramTestContext, TestGeneralPoolMarket) {
    let mut env = presetup().await;

    let test_pool_market = TestGeneralPoolMarket::new();
    test_pool_market
        .init(&mut env.context, &env.registry.keypair.pubkey())
        .await
        .unwrap();

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
    let pool_market_acc = get_account(&mut context, &pool.pool_market).await;
    let pool_market = PoolMarket::unpack_unchecked(&pool_market_acc.data).unwrap();

    assert_eq!(pool_market.manager, test_pool_market.manager.pubkey());

    context.warp_to_slot(3).unwrap();

    let new_manager = Keypair::new();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::update_manager(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool_market.manager.pubkey(),
            &new_manager.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager, &new_manager],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    // Reload pool market
    let pool_market_acc = get_account(&mut context, &pool.pool_market).await;
    let pool_market = PoolMarket::unpack_unchecked(&pool_market_acc.data).unwrap();
    assert_eq!(pool_market.manager, new_manager.pubkey());

    let new_manager = Keypair::new();

    // Try to change back without proper signature
    let tx = Transaction::new_signed_with_payer(
        &[instruction::update_manager(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &test_pool_market.manager.pubkey(),
            &new_manager.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_pool_market.manager, &new_manager],
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
