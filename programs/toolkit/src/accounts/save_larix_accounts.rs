// .subcommand(SubCommand::with_name("save-larix-accounts"))
// pub async fn command_save_larix_accounts(reserve_filepath: &str) -> anyhow::Result<()> {
//     let mut reserve_data = read_file(find_file(reserve_filepath).unwrap());
//     let reserve = Reserve::unpack_from_slice(reserve_data.as_mut_slice()).unwrap();
//     download_account(
//         &reserve.liquidity.supply_pubkey,
//         "larix",
//         "liquidity_supply",
//     )
//     .await;
//     download_account(
//         &reserve.liquidity.fee_receiver,
//         "larix",
//         "liquidity_fee_receiver",
//     )
//     .await;
//     download_account(&reserve.collateral.mint_pubkey, "larix", "collateral_mint").await;
//     download_account(
//         &reserve.collateral.supply_pubkey,
//         "larix",
//         "collateral_supply",
//     )
//     .await;
//     Ok(())
// }

// pub async fn command_save_quarry_accounts(config: &Config) -> anyhow::Result<()> {
//     let mut default_accounts = config.get_default_accounts();
//     // let default_accounts = config.get_default_accounts();
//     let file_path = "../tests/tests/fixtures/quarry/quarry.bin";
//     fs::remove_file(file_path)?;
//     println!("quarry {}", default_accounts.quarry.quarry);
//     download_account(&default_accounts.quarry.quarry, "quarry", "quarry").await;
//     let data: Vec<u8> = read_file(find_file(file_path).unwrap());
//     // first 8 bytes are meta information
//     let adjusted = &data[8..];
//     let deserialized = quarry_mine::Quarry::try_from_slice(adjusted)?;
//     println!("rewarder {}", deserialized.rewarder);
//     println!("token mint {}", deserialized.token_mint_key);
//     default_accounts.quarry.rewarder = deserialized.rewarder;
//     default_accounts.quarry.token_mint = deserialized.token_mint_key;
//     save_config_file::<DefaultAccounts, &str>(&default_accounts, "default.devnet.yaml")?;
//     Ok(())
// }
