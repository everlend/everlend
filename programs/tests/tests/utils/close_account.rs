#![cfg(feature = "test-bpf")]

use solana_program_test::*;

use crate::utils::*;

async fn setup() -> (ProgramTestContext, TestGeneralPoolMarket) {
    let (context, .., test_general_pool_market, _, _) = presetup().await;

    (context, test_general_pool_market)
}

#[tokio::test]
async fn close_account() {
    let (mut context, test_general_pool_market) = setup().await;

    let general_pool = TestGeneralPool::new(&test_general_pool_market, None);
    general_pool
        .create(&mut context, &test_general_pool_market)
        .await
        .unwrap();

    let account_keys = Keypair::new();
    let manager = context.payer.pubkey().clone();

    create_token_account(
        &mut context,
        &account_keys,
        &general_pool.token_mint_pubkey,
        &manager,
        10,
    )
    .await
    .unwrap();

    let account = get_account(&mut context, &account_keys.pubkey()).await;
    assert_ne!(account.lamports, 0);
    spl_token::state::Account::unpack(account.data.as_slice()).unwrap();

    let mut current_slot = 3;
    context.warp_to_slot(current_slot).unwrap();

    crate::utils::close_account(&mut context, &account_keys.pubkey())
        .await
        .unwrap();

    while let Some(_) = context
        .banks_client
        .get_account(account_keys.pubkey())
        .await
        .expect("account not found")
    {
        current_slot += 1;
        context.warp_to_slot(current_slot).unwrap();
    }

    println!("Account closed in slot: {}", current_slot);
}
