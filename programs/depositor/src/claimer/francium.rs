use crate::claimer::RewardClaimer;
use crate::state::MiningType;
use everlend_utils::cpi::francium;
use everlend_utils::{AccountLoader, EverlendError};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use std::{iter::Enumerate, slice::Iter};
use crate::utils::FillRewardAccounts;

/// Container
#[derive(Clone)]
pub struct FranciumClaimer<'a, 'b> {
    farming_pool: &'a AccountInfo<'b>,
    farming_pool_authority: &'a AccountInfo<'b>,
    pool_stake_token: &'a AccountInfo<'b>,
    pool_reward_a: &'a AccountInfo<'b>,
    pool_reward_b: &'a AccountInfo<'b>,
    user_farming: &'a AccountInfo<'b>,
    user_stake: &'a AccountInfo<'b>,
    clock: &'a AccountInfo<'b>,
    sub_reward: &'a AccountInfo<'b>,
}

impl<'a, 'b> FranciumClaimer<'a, 'b> {
    ///
    pub fn init(
        staking_program_id: &Pubkey,
        internal_mining_type: MiningType,
        fill_sub_rewards_accounts: Option<FillRewardAccounts<'a, 'b>>,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<FranciumClaimer<'a, 'b>, ProgramError> {
        let  (farming_pool, staking_program_id_pubkey) =
            match internal_mining_type {
                MiningType::Francium {
                    farming_pool,
                    staking_program_id,
                    ..
                } => (farming_pool, staking_program_id),
                _ => return Err(EverlendError::MiningNotInitialized.into()),
            };

        if !staking_program_id_pubkey.eq(staking_program_id) {
            return Err(ProgramError::InvalidArgument);
        }

        let farming_pool = AccountLoader::next_with_key(account_info_iter, &farming_pool)?;
        let farming_pool_authority = AccountLoader::next_unchecked(account_info_iter)?;
        let pool_stake_token = AccountLoader::next_unchecked(account_info_iter)?;
        let pool_reward_a = AccountLoader::next_unchecked(account_info_iter)?;
        let pool_reward_b = AccountLoader::next_unchecked(account_info_iter)?;
        let user_farming = AccountLoader::next_unchecked(account_info_iter)?;
        let user_stake = AccountLoader::next_unchecked(account_info_iter)?;
        let clock = AccountLoader::next_unchecked(account_info_iter)?;
        let sub_reward = fill_sub_rewards_accounts.as_ref().unwrap().reward_transit_info;

        Ok(FranciumClaimer {
            farming_pool,
            farming_pool_authority,
            pool_stake_token,
            pool_reward_a,
            pool_reward_b,
            user_farming,
            user_stake,
            clock,
            sub_reward,
        })
    }
}

impl<'a, 'b> RewardClaimer<'b> for FranciumClaimer<'a, 'b> {
    ///
    fn claim_reward(
        &self,
        staking_program_id: &Pubkey,
        reward_transit_token_account: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        francium::stake(
            staking_program_id,
            authority.clone(),
            self.user_farming.clone(),
            self.user_stake.clone(),
            reward_transit_token_account.clone(),
            self.sub_reward.clone(),
            self.farming_pool.clone(),
            self.farming_pool_authority.clone(),
            self.pool_stake_token.clone(),
            self.pool_reward_a.clone(),
            self.pool_reward_b.clone(),
            self.clock.clone(),
            0,
            signers_seeds,
        )?;
        Ok(())
    }
}