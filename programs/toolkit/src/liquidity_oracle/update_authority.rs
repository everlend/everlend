// .subcommand(
//     SubCommand::with_name("update-liquidity-oracle-authority")
//         .about("Update liquidity oracle authority")
//         .arg(
//             Arg::with_name("authority")
//                 .long("authority")
//                 .validator(is_keypair)
//                 .value_name("AUTHORITY")
//                 .takes_value(true)
//                 .required(true)
//                 .help("Old manager keypair"),
//         )
//         .arg(
//             Arg::with_name("new-authority")
//                 .long("new-authority")
//                 .validator(is_keypair)
//                 .value_name("NEW-AUTHORITY")
//                 .takes_value(true)
//                 .required(true)
//                 .help("New manager keypair"),
//         ),
// )
