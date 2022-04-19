#![cfg(feature = "test-bpf")]

use solana_program::instruction::InstructionError;
use solana_program::system_instruction::SystemError;
use solana_program_test::*;
use solana_sdk::transaction::{Transaction, TransactionError};
use solana_sdk::{signature::Keypair, signer::Signer};

use everlend_depositor::find_transit_program_address;
use everlend_utils::find_program_address;

use crate::utils::*;

async fn setup() -> (ProgramTestContext, TestDepositor) {
    let (mut context, _, _, registry, general_pool_market, income_pool_market, ..) =
        presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    let test_depositor = TestDepositor::new();
    test_depositor
        .init(
            &mut context,
            &registry,
            &general_pool_market,
            &income_pool_market,
            &test_liquidity_oracle,
        )
        .await
        .unwrap();

    (context, test_depositor)
}

#[tokio::test]
async fn success() {
    let (mut context, test_depositor) = setup().await;

    let token_mint = Keypair::new();
    let payer_pubkey = context.payer.pubkey();

    create_mint(&mut context, &token_mint, &payer_pubkey)
        .await
        .unwrap();

    test_depositor
        .create_transit(&mut context, &token_mint.pubkey(), None)
        .await
        .unwrap();

    let (transit_pubkey, _) = find_transit_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
        &token_mint.pubkey(),
        "",
    );

    let (depositor_authority, _) = find_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
    );

    let transit = get_token_account_data(&mut context, &transit_pubkey).await;

    assert_eq!(transit.mint, token_mint.pubkey());
    assert_eq!(transit.owner, depositor_authority);
}

#[tokio::test]
async fn success_with_different_seed() {
    let (mut context, test_depositor) = setup().await;

    let token_mint = Keypair::new();
    let payer_pubkey = context.payer.pubkey();

    create_mint(&mut context, &token_mint, &payer_pubkey)
        .await
        .unwrap();

    test_depositor
        .create_transit(&mut context, &token_mint.pubkey(), None)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    let (transit_pubkey, _) = find_transit_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
        &token_mint.pubkey(),
        "",
    );

    let (depositor_authority, _) = find_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
    );

    let transit = get_token_account_data(&mut context, &transit_pubkey).await;

    assert_eq!(transit.mint, token_mint.pubkey());
    assert_eq!(transit.owner, depositor_authority);

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::create_transit(
            &everlend_depositor::id(),
            &test_depositor.depositor.pubkey(),
            &token_mint.pubkey(),
            &context.payer.pubkey(),
            Some("second".to_string()),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();

    let (transit_pubkey_second, _) = find_transit_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
        &token_mint.pubkey(),
        "second",
    );

    let transit = get_token_account_data(&mut context, &transit_pubkey_second).await;

    assert_eq!(transit.mint, token_mint.pubkey());
    assert_eq!(transit.owner, depositor_authority);
}

#[tokio::test]
async fn fail_double_create() {
    let (mut context, test_depositor) = setup().await;

    let token_mint = Keypair::new();
    let payer_pubkey = context.payer.pubkey();

    create_mint(&mut context, &token_mint, &payer_pubkey)
        .await
        .unwrap();

    test_depositor
        .create_transit(&mut context, &token_mint.pubkey(), None)
        .await
        .unwrap();

    context.warp_to_slot(3).unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[everlend_depositor::instruction::create_transit(
            &everlend_depositor::id(),
            &test_depositor.depositor.pubkey(),
            &token_mint.pubkey(),
            &context.payer.pubkey(),
            None,
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
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(SystemError::AccountAlreadyInUse as u32),
        )
    );
}
