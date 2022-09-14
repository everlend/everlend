#![deny(missing_docs)]

pub mod state;
pub mod instructions;
pub mod instruction;
pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

pub use solana_program;

solana_program::declare_id!("ELDR7M6m1ysPXks53T7da6zkhnhJV44twXLiAgTf2VpM");
