// .subcommand(SubCommand::with_name("save-quarry-accounts"))

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
