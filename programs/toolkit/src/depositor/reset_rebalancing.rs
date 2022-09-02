// .subcommand(
//     SubCommand::with_name("reset-rebalancing")
//         .about("Reset rebalancing")
//         .arg(
//             Arg::with_name("rebalancing")
//                 .long("rebalancing")
//                 .validator(is_pubkey)
//                 .value_name("ADDRESS")
//                 .takes_value(true)
//                 .required(true)
//                 .help("Rebalancing pubkey"),
//         )
//         .arg(
//             Arg::with_name("amount-to-distribute")
//                 .long("amount-to-distribute")
//                 .validator(is_amount)
//                 .value_name("NUMBER")
//                 .takes_value(true)
//                 .required(true)
//                 .help("Amount to distribute"),
//         )
//         .arg(
//             Arg::with_name("distributed-liquidity")
//                 .long("distributed-liquidity")
//                 .validator(is_amount)
//                 .value_name("NUMBER")
//                 .takes_value(true)
//                 .required(true)
//                 .help("Distributed liduidity"),
//         )
//         .arg(
//             Arg::with_name("distribution")
//                 .long("distribution")
//                 .multiple(true)
//                 .value_name("DISTRIBUTION")
//                 .short("d")
//                 .number_of_values(10)
//                 .required(true)
//                 .takes_value(true),
//         ),
// )
