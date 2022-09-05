#![deny(missing_docs)]

//! Registry contract

pub mod instruction;
pub mod instructions;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("REGQ6c5up9wmY3HRasUDaoJNHgqzSMr4edkuuBVT8Zk");
