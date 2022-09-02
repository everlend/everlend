// .subcommand(
//     SubCommand::with_name("set-pool-config")
//         .about("Create or update pool config")
//         .arg(
//             Arg::with_name("pool")
//                 .long("pool")
//                 .short("P")
//                 .validator(is_pubkey)
//                 .value_name("ADDRESS")
//                 .takes_value(true)
//                 .required(true)
//                 .help("General pool pubkey"),
//         )
//         .arg(
//             Arg::with_name("min-deposit")
//                 .long("min-deposit")
//                 .value_name("NUMBER")
//                 .takes_value(true)
//                 .help("Minimum amount for deposit"),
//         )
//         .arg(
//             Arg::with_name("min-withdraw")
//                 .long("min-withdraw")
//                 .value_name("NUMBER")
//                 .takes_value(true)
//                 .help("Minimum amount for deposit"),
//         ),
// )