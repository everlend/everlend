#![cfg(feature = "test-bpf")]

use everlend_registry::state::{AccountType, SetRegistryConfigParams, TOTAL_DISTRIBUTIONS};
use solana_program::pubkey::Pubkey;
use solana_program_test::*;

use crate::utils::*;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let mut config = SetRegistryConfigParams {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        reward_program_ids: [Pubkey::default(); TOTAL_DISTRIBUTIONS],
        refresh_income_interval: REFRESH_INCOME_INTERVAL,
    };
    config.money_market_program_ids[0] = spl_token_lending::id();

    test_registry
        .set_registry_config(&mut context, config)
        .await
        .unwrap();

    let registry_config = test_registry.get_config_data(&mut context).await;
    assert_eq!(registry_config.account_type, AccountType::RegistryConfig);
}
