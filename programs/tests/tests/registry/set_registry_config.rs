#![cfg(feature = "test-bpf")]

use everlend_registry::state::{
    AccountType, DistributionPubkeys, RegistryPrograms, RegistryRootAccounts, RegistrySettings,
};
use solana_program_test::*;

use crate::utils::*;

#[tokio::test]
async fn success() {
    let mut context = program_test().start_with_context().await;

    let test_registry = TestRegistry::new();
    test_registry.init(&mut context).await.unwrap();

    let mut programs = RegistryPrograms {
        general_pool_program_id: everlend_general_pool::id(),
        ulp_program_id: everlend_ulp::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: DistributionPubkeys::default(),
    };
    programs.money_market_program_ids[0] = spl_token_lending::id();

    test_registry
        .set_registry_config(
            &mut context,
            programs,
            RegistryRootAccounts::default(),
            RegistrySettings {
                refresh_income_interval: REFRESH_INCOME_INTERVAL,
            },
        )
        .await
        .unwrap();

    test_registry
        .set_registry_root_accounts(&mut context, RegistryRootAccounts::default())
        .await
        .unwrap();

    let (config, programs, roots, settings) = test_registry.get_config_data(&mut context).await;
    println!("programs = {:?}", programs);
    println!("roots = {:?}", roots);
    println!("settings = {:?}", settings);
    assert_eq!(config.account_type, AccountType::RegistryConfig);
}
