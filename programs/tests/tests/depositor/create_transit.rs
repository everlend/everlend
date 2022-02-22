#![cfg(feature = "test-bpf")]

use crate::utils::*;
use everlend_depositor::find_transit_program_address;
use everlend_utils::find_program_address;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, signer::Signer};

async fn setup() -> (ProgramTestContext, TestDepositor) {
    let (mut context, _, _, registry) = presetup().await;

    let test_liquidity_oracle = TestLiquidityOracle::new();
    test_liquidity_oracle.init(&mut context).await.unwrap();

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context).await.unwrap();

    let income_pool_market = TestIncomePoolMarket::new();
    income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

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
        .create_transit(&mut context, &token_mint.pubkey())
        .await
        .unwrap();

    let (transit_pubkey, _) = find_transit_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
        &token_mint.pubkey(),
    );

    let (depositor_authority, _) = find_program_address(
        &everlend_depositor::id(),
        &test_depositor.depositor.pubkey(),
    );

    let transit = get_token_account_data(&mut context, &transit_pubkey).await;

    assert_eq!(transit.mint, token_mint.pubkey());
    assert_eq!(transit.owner, depositor_authority);
}
