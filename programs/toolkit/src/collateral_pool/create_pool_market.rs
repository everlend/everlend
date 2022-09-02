// .subcommand(
//     SubCommand::with_name("create-collateral-pool-market")
//         .about("Create a new MM pool market")
//         .arg(
//             Arg::with_name("money-market")
//                 .long("money-market")
//                 .value_name("NUMBER")
//                 .takes_value(true)
//                 .required(true)
//                 .help("Money market index"),
//         )
//         .arg(
//             Arg::with_name("keypair")
//                 .long("keypair")
//                 .validator(is_keypair)
//                 .value_name("KEYPAIR")
//                 .takes_value(true)
//                 .help("Keypair [default: new keypair]"),
//         ),
// )
