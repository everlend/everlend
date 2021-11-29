#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_depositor::state::AccountType;
use solana_program_test::*;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;

    let test_depositor = TestDepositor::new();
    test_depositor.init(&mut context).await.unwrap();

    let depositor = test_depositor.get_data(&mut context).await;

    assert_eq!(depositor.account_type, AccountType::Depositor);
}
