#![deny(missing_docs)]

//! Depositor contract

pub mod claimer;
pub mod instruction;
pub mod instructions;
pub mod money_market;
pub mod processor;
pub mod state;
pub mod utils;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
use everlend_utils::cpi::francium::FRANCIUM_REWARD_SEED;
use everlend_utils::{Seeds, PDA};
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("DepSR26sqzN67TNf1aZ3VCjTPduzKKqTEY8QQkk3KwEz");

/// The list of allowed transit seeds
const ALLOWED_TRANSIT_SEEDS: &[&str] = &["", "lm_reward", "reserve", FRANCIUM_REWARD_SEED];

/// Generates transit address
pub struct TransitPDA<'a> {
    ///
    pub seed: &'a str,
    ///
    pub depositor: Pubkey,
    ///
    pub mint: Pubkey,
}

impl<'a> PDA for TransitPDA<'a> {
    fn get_raw_seeds(&self) -> Seeds {
        Seeds(vec![
            self.seed.as_bytes().to_vec(),
            self.depositor.to_bytes().to_vec(),
            self.mint.to_bytes().to_vec(),
        ])
    }
}

/// Generates rebalancing address
pub struct RebalancingPDA {
    ///
    pub depositor: Pubkey,
    ///
    pub mint: Pubkey,
}

impl PDA for RebalancingPDA {
    fn get_raw_seeds(&self) -> Seeds {
        Seeds(vec![
            "rebalancing".as_bytes().to_vec(),
            self.depositor.to_bytes().to_vec(),
            self.mint.to_bytes().to_vec(),
        ])
    }
}

/// Generates internal mining program address
pub struct InternalMiningPDA {
    ///
    pub liquidity_mint: Pubkey,
    // Money market collateral mint
    ///
    pub collateral_mint: Pubkey,
    ///
    pub depositor: Pubkey,
}

impl PDA for InternalMiningPDA {
    fn get_raw_seeds(&self) -> Seeds {
        Seeds(vec![
            "internal_mining".as_bytes().to_vec(),
            self.liquidity_mint.to_bytes().to_vec(),
            self.collateral_mint.to_bytes().to_vec(),
            self.depositor.to_bytes().to_vec(),
        ])
    }
}
