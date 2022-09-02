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
