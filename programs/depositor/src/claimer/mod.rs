//! Money markets claimers

use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

mod larix;
mod port_finance;
mod quarry;
mod francium;

pub use larix::*;
pub use port_finance::*;
pub use quarry::*;
pub use francium::*;
///
pub trait RewardClaimer<'a> {
    /// Claim mining reward
    fn claim_reward(
        &self,
        staking_program_id: &Pubkey,
        reward_transit_token_account: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError>;
}
