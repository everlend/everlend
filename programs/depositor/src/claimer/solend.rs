use crate::claimer::RewardClaimer;
use crate::state::MiningType;
use borsh::BorshDeserialize;
use everlend_utils::cpi::solend;
use everlend_utils::{AccountLoader, EverlendError};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use std::{iter::Enumerate, slice::Iter};

/// Container
#[derive(Clone)]
pub struct SolendClaimer<'a, 'b> {
    distributor: &'a AccountInfo<'b>,
    claim_status: &'a AccountInfo<'b>,
    source: &'a AccountInfo<'b>,
    executor: &'a AccountInfo<'b>,
    claim_data: solend::ClaimData,
}

impl<'a, 'b> SolendClaimer<'a, 'b> {
    ///
    pub fn init(
        staking_program_id: &Pubkey,
        internal_mining_type: MiningType,
        additional_data: &[u8],
        executor: &'a AccountInfo<'b>,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<SolendClaimer<'a, 'b>, ProgramError> {
        if !staking_program_id.eq(&solend::staking_program_id()) {
            return Err(ProgramError::InvalidArgument);
        }

        match internal_mining_type {
            MiningType::Solend { .. } => {}
            _ => return Err(EverlendError::MiningNotInitialized.into()),
        };

        let distributor = AccountLoader::next_with_owner(account_info_iter, staking_program_id)?;
        let claim_status = AccountLoader::next_uninitialized(account_info_iter)?;
        let source = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let claim_data = solend::ClaimData::try_from_slice(additional_data)?;

        Ok(SolendClaimer {
            distributor,
            claim_status,
            source,
            executor,
            claim_data,
        })
    }
}

impl<'a, 'b> RewardClaimer<'b> for SolendClaimer<'a, 'b> {
    ///
    fn claim_reward(
        &self,
        staking_program_id: &Pubkey,
        reward_transit_token_account: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        solend::claim_rewards(
            staking_program_id,
            self.distributor.clone(),
            self.claim_status.clone(),
            self.source.clone(),
            reward_transit_token_account,
            authority.clone(),
            self.executor.clone(),
            self.claim_data.clone(),
            signers_seeds,
        )?;
        Ok(())
    }
}
