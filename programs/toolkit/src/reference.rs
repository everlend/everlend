// let _ = match matches.subcommand() {
//     ("init-mining", Some(arg_matches)) => {
//         let staking_money_market =
//             value_of::<usize>(arg_matches, "staking-money-market").unwrap();
//         let token = value_of::<String>(arg_matches, "token").unwrap();
//         let sub_reward_mint = pubkey_of(arg_matches, "sub-reward-mint");
//         command_init_mining(
//             &config,
//             StakingMoneyMarket::from(staking_money_market),
//             &token,
//             sub_reward_mint,
//         )
//     }

//     ("save-larix-accounts", Some(_)) => {
//         command_save_larix_accounts("../tests/tests/fixtures/larix/reserve_sol.bin").await
//     }
//     ("test-larix-mining-raw", Some(_)) => command_test_larix_mining_raw(&config),
//     ("save-quarry-accounts", Some(_)) => command_save_quarry_accounts(&config).await,
//     ("init-quarry-mining-accounts", Some(arg_matches)) => {
//         let token = value_of::<String>(arg_matches, "token").unwrap();
//         command_init_quarry_mining_accounts(&config, &token)
//     }
//     ("test-quarry-mining-raw", Some(arg_matches)) => {
//         let token = value_of::<String>(arg_matches, "token").unwrap();
//         command_test_quarry_mining_raw(&config, &token)
//     }
//     ("set-pool-config", Some(arg_matches)) => {
//         let pool = pubkey_of(arg_matches, "pool").unwrap();
//         let deposit_minimum = value_of::<u64>(arg_matches, "min-deposit");
//         let withdraw_minimum = value_of::<u64>(arg_matches, "min-withdraw");
//         let params = SetPoolConfigParams {
//             deposit_minimum,
//             withdraw_minimum,
//         };

//         command_set_pool_config(&config, pool, params).await
//     }
//     ("create-general-pool-market", Some(arg_matches)) => {
//         let keypair = keypair_of(arg_matches, "keypair");
//         let registry_pubkey = pubkey_of(arg_matches, "registry").unwrap();
//         command_create_general_pool_market(&config, keypair, registry_pubkey).await
//     }
//     ("create-income-pool-market", Some(arg_matches)) => {
//         let keypair = keypair_of(arg_matches, "keypair");
//         command_create_income_pool_market(&config, keypair).await
//     }
//     ("create--pool-market", Some(arg_matches)) => {
//         let keypair = keypair_of(arg_matches, "keypair");
//         let money_market = value_of::<usize>(arg_matches, "money-market").unwrap();
//         command_create_collateral_pool_market(&config, keypair, MoneyMarket::from(money_market))
//             .await
//     }
//     ("create-liquidity-oracle", Some(arg_matches)) => {
//         let keypair = keypair_of(arg_matches, "keypair");
//         command_create_liquidity_oracle(&config, keypair).await
//     }
//     ("update-liquidity-oracle-authority", Some(arg_matches)) => {
//         let authority = keypair_of(arg_matches, "authority").unwrap();
//         let new_authority = keypair_of(arg_matches, "new-authority").unwrap();

//         command_update_liquidity_oracle(&config, authority, new_authority).await
//     }
//     ("create-depositor", Some(arg_matches)) => {
//         let keypair = keypair_of(arg_matches, "keypair");
//         let executor_pubkey = pubkey_of(arg_matches, "rebalance-executor").unwrap();
//         command_create_depositor(&config, keypair, executor_pubkey).await
//     }
//     ("create-collateral-pool", Some(arg_matches)) => {
//         let money_market = value_of::<usize>(arg_matches, "money-market").unwrap();
//         let mints: Vec<_> = arg_matches.values_of("mints").unwrap().collect();
//         command_create_collateral_pool(&config, MoneyMarket::from(money_market), mints).await
//     }
//     ("create-collateral-pools", Some(arg_matches)) => {
//         let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
//         command_create_collateral_pools(&config, accounts_path).await
//     }
//     ("create-pool-withdraw-authority", Some(arg_matches)) => {
//         let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
//         create_pool_withdraw_authority(&config, accounts_path).await
//     }
//     ("create-token-accounts", Some(arg_matches)) => {
//         let mints: Vec<_> = arg_matches.values_of("mints").unwrap().collect();
//         command_create_token_accounts(&config, mints).await
//     }
//     ("add-reserve-liquidity", Some(arg_matches)) => {
//         let mint = arg_matches.value_of("mint").unwrap();
//         let amount = value_of::<u64>(arg_matches, "amount").unwrap();
//         command_add_reserve_liquidity(&config, mint, amount).await
//     }
//     ("cancel-withdraw-request", Some(arg_matches)) => {
//         let request_pubkey = pubkey_of(arg_matches, "request").unwrap();
//         command_cancel_withdraw_request(&config, &request_pubkey).await
//     }
//     ("reset-rebalancing", Some(arg_matches)) => {
//         let rebalancing_pubkey = pubkey_of(arg_matches, "rebalancing").unwrap();
//         let amount_to_distribute =
//             value_of::<u64>(arg_matches, "amount-to-distribute").unwrap();
//         let distributed_liquidity =
//             value_of::<u64>(arg_matches, "distributed-liquidity").unwrap();
//         let distribution: Vec<u64> = values_of::<u64>(arg_matches, "distribution").unwrap();
//         command_reset_rebalancing(
//             &config,
//             &rebalancing_pubkey,
//             amount_to_distribute,
//             distributed_liquidity,
//             distribution,
//         )
//         .await
//     }
//     ("info-reserve-liquidity", Some(_)) => command_info_reserve_liquidity(&config).await,
//     ("create", Some(arg_matches)) => {
//         let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
//         let mints: Vec<_> = arg_matches.values_of("mints").unwrap().collect();
//         let rebalance_executor_pubkey = pubkey_of(arg_matches, "rebalance-executor").unwrap();
//         command_create(&config, accounts_path, mints, rebalance_executor_pubkey).await
//     }
//     ("info", Some(arg_matches)) => {
//         let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
//         command_info(&config, accounts_path).await
//     }
//     ("test", Some(arg_matches)) => {
//         let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
//         let case = value_of::<String>(arg_matches, "case");
//         commands_test::command_run_test(&config, accounts_path, case).await
//     }
//     ("multisig", Some(arg_matches)) => {
//         let _ = match arg_matches.subcommand() {
//             ("create", Some(arg_matches)) => {
//                 let owners: Vec<_> = arg_matches
//                     .values_of("owners")
//                     .unwrap()
//                     .map(|str| Pubkey::from_str(str).unwrap())
//                     .collect();
//                 let threshold = value_of::<u64>(arg_matches, "threshold").unwrap();

//                 commands_multisig::command_create_multisig(&config, owners, threshold).await
//             }
//             ("propose-upgrade", Some(arg_matches)) => {
//                 let program_pubkey = pubkey_of(arg_matches, "program").unwrap();
//                 let buffer_pubkey = pubkey_of(arg_matches, "buffer").unwrap();
//                 let spill_pubkey = pubkey_of(arg_matches, "spill").unwrap();
//                 let multisig_pubkey = pubkey_of(arg_matches, "multisig").unwrap();

//                 commands_multisig::command_propose_upgrade(
//                     &config,
//                     &program_pubkey,
//                     &buffer_pubkey,
//                     &spill_pubkey,
//                     &multisig_pubkey,
//                 )
//                 .await
//             }
//             ("approve", Some(arg_matches)) => {
//                 let transaction_pubkey = pubkey_of(arg_matches, "transaction").unwrap();
//                 let multisig_pubkey = pubkey_of(arg_matches, "multisig").unwrap();

//                 commands_multisig::command_approve(
//                     &config,
//                     &multisig_pubkey,
//                     &transaction_pubkey,
//                 )
//                 .await
//             }
//             ("execute", Some(arg_matches)) => {
//                 let transaction_pubkey = pubkey_of(arg_matches, "transaction").unwrap();
//                 let multisig_pubkey = pubkey_of(arg_matches, "multisig").unwrap();

//                 commands_multisig::command_execute_transaction(
//                     &config,
//                     &multisig_pubkey,
//                     &transaction_pubkey,
//                 )
//                 .await
//             }
//             ("info", Some(arg_matches)) => {
//                 let multisig_pubkey = pubkey_of(arg_matches, "multisig").unwrap();

//                 commands_multisig::command_info_multisig(&config, &multisig_pubkey).await
//             }
//             _ => unreachable!(),
//         }
//         .map_err(|err| {
//             eprintln!("{}", err);
//             exit(1);
//         });

//         Ok(())
//     }

//     // TODO remove after migration
//     ("create-safety-fund-token-account", Some(arg_matches)) => {
//         let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
//         let case = value_of::<String>(arg_matches, "case");
//         command_create_income_pool_safety_fund_token_account(&config, accounts_path, case).await
//     }
//     ("create-depositor-transit-token-account", Some(arg_matches)) => {
//         let token_mint = pubkey_of(arg_matches, "token-mint").unwrap();
//         let seed = value_of::<String>(arg_matches, "seed");
//         command_create_depositor_transit_account(&config, token_mint, seed).await
//     }
//     ("update-manager", Some(arg_matches)) => {
//         let program = value_of::<String>(arg_matches, "program").unwrap();
//         let accounts_path = arg_matches.value_of("accounts").unwrap_or("accounts.yaml");
//         let source = keypair_of(arg_matches, "source").unwrap();
//         let target = keypair_of(arg_matches, "target").unwrap();

//         command_update_manager(&config, accounts_path, program, source, target).await
//     }
//     _ => unreachable!(),
// }
// .map_err(|err| {
//     eprintln!("{}", err);
//     exit(1);
// });
