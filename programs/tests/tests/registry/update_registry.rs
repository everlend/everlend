use everlend_registry::{
    instructions::UpdateRegistryData,
    state::{
        AccountType, DistributionPubkeys, RegistryPrograms, RegistryRootAccounts, RegistrySettings,
    },
};
use solana_program::{example_mocks::solana_sdk::signature::Keypair, pubkey::Pubkey};
use solana_program_test::*;

use crate::utils::*;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let mut mm_program_ids = DistributionPubkeys::default();
    mm_program_ids[0] = spl_token_lending::id();

    let data = UpdateRegistryData {
        general_pool_market: Some(Keypair::new().pubkey()),
        income_pool_market: Some(Keypair::new().pubkey()),
        liquidity_oracle: Some(Keypair::new().pubkey()),
        liquidity_oracle_manager: Some(Keypair::new().pubkey()),
        money_market_program_ids: Some(mm_program_ids),
        collateral_pool_markets: Some(mm_program_ids),
        refresh_income_interval: Some(REFRESH_INCOME_INTERVAL),
    };

    test_registry
        .update_registry(&mut context, data)
        .await
        .unwrap();

    test_registry
        .set_registry_root_accounts(&mut context, RegistryRootAccounts::default())
        .await
        .unwrap();

    let r = test_registry.get_data(&mut context).await;
    println!("data = {:?}", r);
    assert_eq!(r.account_type, AccountType::RegistryConfig);
    assert_eq!(r.general_pool_market, data.general_pool_market.ok());
    assert_eq!(r.income_pool_market, data.income_pool_market.ok());
    assert_eq!(r.liquidity_oracle, data.liquidity_oracle.ok());
    assert_eq!(
        r.liquidity_oracle_manager,
        data.liquidity_oracle_manager.ok()
    );
    assert_eq!(
        r.money_market_program_ids,
        data.money_market_program_ids.ok()
    );
    assert_eq!(r.collateral_pool_markets, data.collateral_pool_markets.ok());
    assert_eq!(r.refresh_income_interval, data.refresh_income_interval.ok());
}
