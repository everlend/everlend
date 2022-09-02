// .subcommand(
//     SubCommand::with_name("create-depositor")
//         .about("Create a new depositor")
//         .arg(
//             Arg::with_name("keypair")
//                 .long("keypair")
//                 .validator(is_keypair)
//                 .value_name("KEYPAIR")
//                 .takes_value(true)
//                 .help("Keypair [default: new keypair]"),
//         )
//         .arg(
//             Arg::with_name("rebalance-executor")
//                 .long("rebalance-executor")
//                 .validator(is_pubkey)
//                 .value_name("PUBKEY")
//                 .takes_value(true)
//                 .help("Rebalance executor pubkey"),
//         ),
// )
