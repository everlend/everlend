// .subcommand(
//     SubCommand::with_name("create")
//         .about("Create a new accounts")
//         .arg(
//             Arg::with_name("mints")
//                 .multiple(true)
//                 .long("mints")
//                 .short("m")
//                 .required(true)
//                 .min_values(1)
//                 .takes_value(true),
//         )
//         .arg(
//             Arg::with_name("accounts")
//                 .short("A")
//                 .long("accounts")
//                 .value_name("PATH")
//                 .takes_value(true)
//                 .help("Accounts file"),
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
