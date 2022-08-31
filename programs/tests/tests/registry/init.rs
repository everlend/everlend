use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use everlend_registry::instruction;
use everlend_registry::state::AccountType;

use crate::utils::*;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let registry = test_registry.get_data(&mut context).await;

    assert_eq!(registry.account_type, AccountType::Registry);
}

#[tokio::test]
async fn fail_second_time_init() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let registry = test_registry.get_data(&mut context).await;

    assert_eq!(registry.account_type, AccountType::Registry);

    context.warp_to_slot(3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[instruction::init(
            &everlend_registry::id(),
            &test_registry.keypair.pubkey(),
            &test_registry.manager.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, InstructionError::AccountAlreadyInitialized)
    );
}
