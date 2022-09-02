// .subcommand(
//     SubCommand::with_name("multisig")
//         .about("Multisig")
//         .subcommand(
//             SubCommand::with_name("create")
//                 .about("Create a new multisig")
//                 .arg(
//                     Arg::with_name("owners")
//                         .multiple(true)
//                         .long("owners")
//                         .required(true)
//                         .min_values(1)
//                         .takes_value(true),
//                 )
//                 .arg(
//                     Arg::with_name("threshold")
//                         .short("th")
//                         .long("threshold")
//                         .value_name("NUMBER")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Threshold"),
//                 ),
//         )
//         .subcommand(
//             SubCommand::with_name("propose-upgrade")
//                 .about("Propose program upgrade")
//                 .arg(
//                     Arg::with_name("program")
//                         .long("program")
//                         .validator(is_pubkey)
//                         .value_name("ADDRESS")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Program pubkey"),
//                 )
//                 .arg(
//                     Arg::with_name("buffer")
//                         .long("buffer")
//                         .validator(is_pubkey)
//                         .value_name("ADDRESS")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Buffer pubkey"),
//                 )
//                 .arg(
//                     Arg::with_name("spill")
//                         .long("spill")
//                         .validator(is_pubkey)
//                         .value_name("ADDRESS")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Spill pubkey"),
//                 )
//                 .arg(
//                     Arg::with_name("multisig")
//                         .long("multisig")
//                         .validator(is_pubkey)
//                         .value_name("ADDRESS")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Multisig pubkey"),
//                 ),
//         )
//         .subcommand(
//             SubCommand::with_name("approve")
//                 .about("Approve transaction")
//                 .arg(
//                     Arg::with_name("transaction")
//                         .long("transaction")
//                         .short("tx")
//                         .validator(is_pubkey)
//                         .value_name("ADDRESS")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Transaction account pubkey"),
//                 )
//                 .arg(
//                     Arg::with_name("multisig")
//                         .long("multisig")
//                         .validator(is_pubkey)
//                         .value_name("ADDRESS")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Multisig pubkey"),
//                 ),
//         )
//         .subcommand(
//             SubCommand::with_name("execute")
//                 .about("Execute transaction")
//                 .arg(
//                     Arg::with_name("transaction")
//                         .long("transaction")
//                         .validator(is_pubkey)
//                         .value_name("ADDRESS")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Transaction account pubkey"),
//                 )
//                 .arg(
//                     Arg::with_name("multisig")
//                         .long("multisig")
//                         .validator(is_pubkey)
//                         .value_name("ADDRESS")
//                         .takes_value(true)
//                         .required(true)
//                         .help("Multisig pubkey"),
//                 ),
//         )
//         .subcommand(
//             SubCommand::with_name("info").about("Multisig info").arg(
//                 Arg::with_name("multisig")
//                     .validator(is_pubkey)
//                     .value_name("ADDRESS")
//                     .takes_value(true)
//                     .required(true)
//                     .help("Multisig pubkey"),
//             ),
//         ),
// )

// pub async fn command_create_multisig(
//     config: &Config,
//     owners: Vec<Pubkey>,
//     threshold: u64,
// ) -> anyhow::Result<()> {
//     println!("owners = {:#?}", owners);
//     println!("threshold = {:?}", threshold);

//     let (multisig_pubkey, multisig_pda) =
//         multisig::create_multisig(config, None, owners, threshold)?;

//     println!("multisig_pubkey = {:?}", multisig_pubkey);
//     println!("multisig_pda = {:?}", multisig_pda);

//     Ok(())
// }

// pub async fn command_info_multisig(
//     config: &Config,
//     multisig_pubkey: &Pubkey,
// ) -> anyhow::Result<()> {
//     let multisig = config.get_account_deserialize::<serum_multisig::Multisig>(multisig_pubkey)?;

//     println!("Owners: {:?}", multisig.owners);
//     println!("Threshold: {:?}", multisig.threshold);

//     println!("Transactions:");
//     let txs: Vec<(Pubkey, serum_multisig::Transaction)> =
//         get_transaction_program_accounts(config, multisig_pubkey)?
//             .into_iter()
//             .filter_map(|(address, account)| {
//                 let mut data_ref = &account.data[..];
//                 match serum_multisig::Transaction::try_deserialize(&mut data_ref) {
//                     Ok(tx) => Some((address, tx)),
//                     _ => None,
//                 }
//             })
//             .collect();

//     for (pubkey, tx) in txs {
//         println!("{:?}", pubkey);
//         println!("Data: {:?}", tx.data);
//         println!("Signers: {:?}", tx.signers);
//         println!("Set seqno: {:?}", tx.owner_set_seqno);
//         println!("Executed: {:?}", tx.did_execute);
//     }

//     Ok(())
// }

// pub async fn command_propose_upgrade(
//     config: &Config,
//     program_pubkey: &Pubkey,
//     buffer_pubkey: &Pubkey,
//     spill_pubkey: &Pubkey,
//     multisig_pubkey: &Pubkey,
// ) -> anyhow::Result<()> {
//     let default_accounts = config.get_default_accounts();
//     let (pda, _) =
//         get_multisig_program_address(&default_accounts.multisig_program_id, multisig_pubkey);

//     let upgrade_instruction =
//         bpf_loader_upgradeable::upgrade(program_pubkey, buffer_pubkey, &pda, spill_pubkey);

//     let transaction_pubkey =
//         multisig::create_transaction(config, multisig_pubkey, upgrade_instruction)?;

//     println!("transaction_pubkey = {:?}", transaction_pubkey);

//     Ok(())
// }

// pub async fn command_approve(
//     config: &Config,
//     multisig_pubkey: &Pubkey,
//     transaction_pubkey: &Pubkey,
// ) -> anyhow::Result<()> {
//     println!("transaction_pubkey = {:#?}", transaction_pubkey);
//     println!("multisig_pubkey = {:?}", multisig_pubkey);

//     let signature = multisig::approve(config, multisig_pubkey, transaction_pubkey)?;

//     println!("signature = {:?}", signature);

//     Ok(())
// }

// pub async fn command_execute_transaction(
//     config: &Config,
//     multisig_pubkey: &Pubkey,
//     transaction_pubkey: &Pubkey,
// ) -> anyhow::Result<()> {
//     println!("transaction_pubkey = {:#?}", transaction_pubkey);
//     println!("multisig_pubkey = {:?}", multisig_pubkey);

//     let signature = multisig::execute_transaction(config, multisig_pubkey, transaction_pubkey)?;

//     println!("signature = {:?}", signature);

//     Ok(())
// }
