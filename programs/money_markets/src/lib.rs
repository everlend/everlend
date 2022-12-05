#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;
pub mod claimer;
pub mod money_market;
pub mod instruction;
pub mod instructions;
pub mod processor;

solana_program::declare_id!("8ysseUUzNAxJgcdYo1KyCSZEsUX88EbzpyzzsExhF6Yp");


