use crate::claimer::RewardClaimer;
use crate::find_transit_program_address;
use crate::state::MiningType;
use crate::utils::FillRewardAccounts;
use borsh::BorshDeserialize;
use everlend_utils::cpi::francium;
use everlend_utils::{assert_account_key, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{
        clock::{self, Clock},
        Sysvar,
    },
};
use std::{iter::Enumerate, slice::Iter};

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
        program_id: &Pubkey,
        staking_program_id: &Pubkey,
        depositor_authority: &Pubkey,
        depositor: &Pubkey,
        internal_mining_type: MiningType,
        fill_sub_rewards_accounts: Option<FillRewardAccounts<'a, 'b>>,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<FranciumClaimer<'a, 'b>, ProgramError> {
        assert_eq!(staking_program_id, &francium::get_staking_program_id());
        let (farming_pool, user_stake_token_account, user_reward_b, user_reward_a) =
            match internal_mining_type {
                MiningType::Francium {
                    farming_pool,
                    user_stake_token_account,
                    user_reward_b,
                    user_reward_a,
                } => (
                    farming_pool,
                    user_stake_token_account,
                    user_reward_b,
                    user_reward_a,
                ),
                _ => return Err(EverlendError::MiningNotInitialized.into()),
            };

        let farming_pool = AccountLoader::next_with_key(account_info_iter, &farming_pool)?;
        let farming_pool_authority = AccountLoader::next_unchecked(account_info_iter)?;
        let pool_stake_token = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let pool_reward_a = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let pool_reward_b = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let user_farming_info = francium::find_user_farming_address(
            depositor_authority,
            farming_pool.key,
            &user_stake_token_account,
        );

        let user_farming = AccountLoader::next_with_key(account_info_iter, &user_farming_info)?;
        let user_stake =
            AccountLoader::next_with_key(account_info_iter, &user_stake_token_account)?;
        let clock = AccountLoader::next_with_key(account_info_iter, &clock::id())?;
        let sub_reward: &AccountInfo;

        let farming_pool_unpack: francium::FarmingPool =
            francium::FarmingPool::try_from_slice(&farming_pool.data.borrow())?;

        if farming_pool_unpack.is_dual_rewards && fill_sub_rewards_accounts.is_some() {
            sub_reward = fill_sub_rewards_accounts
                .as_ref()
                .unwrap()
                .reward_transit_info;
        } else {
            let current_slot = Clock::from_account_info(clock)?.slot;
            if farming_pool_unpack.rewards_per_day != 0
                && farming_pool_unpack.rewards_start_slot != farming_pool_unpack.rewards_end_slot
                && current_slot < farming_pool_unpack.rewards_end_slot
            {
                sub_reward = AccountLoader::next_with_key(account_info_iter, &user_reward_b)?;

                let (user_reward_b_check, _) = find_transit_program_address(
                    program_id,
                    depositor,
                    &farming_pool_unpack.rewards_token_mint_b,
                    francium::FRANCIUM_REWARD_SEED,
                );

                assert_account_key(&sub_reward, &user_reward_b_check)?;
            } else {
                sub_reward = AccountLoader::next_with_key(account_info_iter, &user_reward_a)?;

                let (user_reward_a_check, _) = find_transit_program_address(
                    program_id,
                    depositor,
                    &farming_pool_unpack.rewards_token_mint,
                    francium::FRANCIUM_REWARD_SEED,
                );

                assert_account_key(&sub_reward, &user_reward_a_check)?;
            }
        }

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
