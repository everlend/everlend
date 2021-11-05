#![cfg(feature = "test-bpf")]

use crate::utils::*;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer};

async fn setup() -> (ProgramTestContext, TestDepositor) {
    let mut context = program_test().start_with_context().await;

    let test_depositor = TestDepositor::new(None);
    test_depositor.init(&mut context).await.unwrap();

    (context, test_depositor)
}

#[tokio::test]
async fn success() {
    let (mut context, test_depositor) = setup().await;

    let token_mint = Keypair::new();

    create_mint(
        &mut context,
        &token_mint,
        &test_depositor.rebalancer.pubkey(),
    )
    .await
    .unwrap();

    test_depositor
        .create_transit(&mut context, &token_mint.pubkey())
        .await
        .unwrap();

    // assert_eq!(
    //     get_token_balance(&mut context, &user.destination).await,
    //     100,
    // );
}
