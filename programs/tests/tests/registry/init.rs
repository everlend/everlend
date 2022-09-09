use everlend_registry::state::AccountType;
use solana_program::instruction::InstructionError;
use solana_program_test::*;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::TransactionError;

use crate::utils::*;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let registry = test_registry.get_data(&mut context).await;

    assert_eq!(registry.account_type, AccountType::Registry);
    assert_eq!(registry.manager, test_registry.manager.pubkey());
}

#[tokio::test]
async fn fail_second_time_init() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let registry = test_registry.get_data(&mut context).await;

    assert_eq!(registry.account_type, AccountType::Registry);

    context.warp_to_slot(3).unwrap();

    let err = test_registry.init(&mut context).await.unwrap_err().unwrap();

    assert_eq!(
        err,
        TransactionError::InstructionError(1, InstructionError::AccountAlreadyInitialized)
    );
}
