use everlend_registry::state::{DistributionPubkeys, MoneyMarket};
use everlend_registry::{
    instructions::{UpdateRegistryData, UpdateRegistryMarketsData},
    state::{AccountType, MoneyMarkets},
};
use solana_program::example_mocks::solana_sdk::signature::Keypair;
use solana_program_test::*;

use crate::utils::*;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let mut mm_program_ids = MoneyMarkets::default();
    mm_program_ids[0] = MoneyMarket {
        id: everlend_utils::integrations::MoneyMarket::PortFinance,
        program_id: Keypair::new().pubkey(),
        lending_market: Keypair::new().pubkey(),
    };
    mm_program_ids[1] = MoneyMarket {
        id: everlend_utils::integrations::MoneyMarket::PortFinance,
        program_id: Keypair::new().pubkey(),
        lending_market: Keypair::new().pubkey(),
    };

    let mut collateral_program_ids = DistributionPubkeys::default();
    collateral_program_ids[0] = Keypair::new().pubkey();
    collateral_program_ids[1] = Keypair::new().pubkey();

    let data = UpdateRegistryData {
        general_pool_market: Some(Keypair::new().pubkey()),
        income_pool_market: Some(Keypair::new().pubkey()),
        liquidity_oracle: Some(Keypair::new().pubkey()),
        refresh_income_interval: Some(REFRESH_INCOME_INTERVAL),
    };

    test_registry
        .update_registry(&mut context, data.clone())
        .await
        .unwrap();

    let market_data = UpdateRegistryMarketsData {
        money_markets: Some(mm_program_ids),
        collateral_pool_markets: Some(collateral_program_ids),
    };

    test_registry
        .update_registry_markets(&mut context, market_data.clone())
        .await
        .unwrap();

    let r = test_registry.get_data(&mut context).await;
    let rm = test_registry.get_registry_markets(&mut context).await;

    println!("data = {:?}", r);
    println!("markets = {:?}", rm);

    assert_eq!(r.account_type, AccountType::Registry);
    assert_eq!(r.general_pool_market, data.general_pool_market.unwrap());
    assert_eq!(r.income_pool_market, data.income_pool_market.unwrap());
    assert_eq!(r.liquidity_oracle, data.liquidity_oracle.unwrap());
    assert_eq!(
        r.refresh_income_interval,
        data.refresh_income_interval.unwrap()
    );

    assert_eq!(rm.money_markets, market_data.money_markets.unwrap());
    assert_eq!(
        rm.collateral_pool_markets,
        market_data.collateral_pool_markets.unwrap()
    );
}
