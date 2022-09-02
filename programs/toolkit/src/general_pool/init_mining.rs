// SubCommand::with_name("init-mining")
//                 .arg(
//                     Arg::with_name("staking-money-market")
//                         .long("staking-money-market")
//                         .value_name("NUMBER")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Money market index"),
//                 )
//                 .arg(
//                     Arg::with_name("token")
//                         .long("token")
//                         .short("t")
//                         .value_name("TOKEN")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Token"),
//                 )
//                 .arg(
//                     Arg::with_name("sub-reward-mint")
//                         .long("sub-reward-mint")
//                         .short("m")
//                         .value_name("REWARD_MINT")
//                         .takes_value(true)
//                         .help("Sub reward token mint"),
//                 ),

// pub fn command_init_mining(
//     config: &Config,
//     staking_money_market: StakingMoneyMarket,
//     token: &String,
//     sub_reward_token_mint: Option<Pubkey>,
// ) -> anyhow::Result<()> {
//     let liquidity_miner_option: Option<Box<dyn LiquidityMiner>> = match staking_money_market {
//         StakingMoneyMarket::PortFinance => Some(Box::new(PortLiquidityMiner {})),
//         StakingMoneyMarket::Larix => Some(Box::new(LarixLiquidityMiner {})),
//         StakingMoneyMarket::Quarry => Some(Box::new(QuarryLiquidityMiner {})),
//         _ => None,
//     };

//     if liquidity_miner_option.is_none() {
//         return Err(anyhow::anyhow!("Wrong staking money market"));
//     }
//     let liquidity_miner = liquidity_miner_option.unwrap();
//     let mut mining_pubkey = liquidity_miner.get_mining_pubkey(config, token);
//     if mining_pubkey.eq(&Pubkey::default()) {
//         let new_mining_account = Keypair::new();
//         mining_pubkey = new_mining_account.pubkey();
//         liquidity_miner.create_mining_account(
//             config,
//             token,
//             &new_mining_account,
//             sub_reward_token_mint,
//         )?;
//     };
//     let pubkeys = liquidity_miner.get_pubkeys(config, token);
//     let mining_type =
//         liquidity_miner.get_mining_type(config, token, mining_pubkey, sub_reward_token_mint);
//     execute_init_mining_accounts(config, &pubkeys.unwrap(), mining_type)?;
//     let money_market = match staking_money_market {
//         StakingMoneyMarket::Larix => MoneyMarket::Larix,
//         StakingMoneyMarket::Solend => MoneyMarket::Solend,
//         _ => MoneyMarket::PortFinance,
//     };
//     save_mining_accounts(config, token, money_market, &config.network)?;
//     Ok(())
// }
