// .subcommand(
//     SubCommand::with_name("create-depositor-transit-token-account")
//         .about("Run create depositor transit token account")
//         .arg(
//             Arg::with_name("seed")
//                 .long("seed")
//                 .value_name("SEED")
//                 .takes_value(true)
//                 .help("Transit seed"),
//         )
//         .arg(
//             Arg::with_name("token-mint")
//                 .long("token-mint")
//                 .value_name("MINT")
//                 .validator(is_pubkey)
//                 .takes_value(true)
//                 .help("Rewards token mint"),
//         ),
// )
