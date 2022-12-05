use crate::claimer::RewardClaimer;
use everlend_depositor::state::MiningType;
use everlend_depositor::utils::FillRewardAccounts;
use everlend_utils::cpi::port_finance;
use everlend_utils::{AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, sysvar::clock,
};
use std::{iter::Enumerate, slice::Iter};

/// Container
#[derive(Clone)]
pub struct PortFinanceClaimer<'a, 'b> {
    stake_account: &'a AccountInfo<'b>,
    staking_pool: &'a AccountInfo<'b>,
    staking_pool_authority: &'a AccountInfo<'b>,
    reward_token_pool: &'a AccountInfo<'b>,
    clock: &'a AccountInfo<'b>,
    sub_reward: Option<(&'a AccountInfo<'b>, &'a AccountInfo<'b>)>,
}

impl<'a, 'b> PortFinanceClaimer<'a, 'b> {
    ///
    pub fn init(
        staking_program_id: &Pubkey,
        internal_mining_type: MiningType,
        with_subrewards: bool,
        fill_sub_rewards_accounts: Option<FillRewardAccounts<'a, 'b>>,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<PortFinanceClaimer<'a, 'b>, ProgramError> {
        // Parse mining  accounts if presented
        let (staking_account_pubkey, staking_pool_pubkey, staking_program_id_pubkey) =
            match internal_mining_type {
                MiningType::PortFinance {
                    staking_account,
                    staking_pool,
                    staking_program_id,
                    ..
                } => (staking_account, staking_pool, staking_program_id),
                _ => return Err(EverlendError::MiningNotInitialized.into()),
            };

        if !staking_program_id_pubkey.eq(staking_program_id) {
            return Err(ProgramError::InvalidArgument);
        }

        let stake_account =
            AccountLoader::next_with_key(account_info_iter, &staking_account_pubkey)?;
        let staking_pool = AccountLoader::next_with_key(account_info_iter, &staking_pool_pubkey)?;
        let staking_pool_authority = AccountLoader::next_unchecked(account_info_iter)?;

        let reward_token_pool = AccountLoader::next_unchecked(account_info_iter)?;

        let clock = AccountLoader::next_with_key(account_info_iter, &clock::id())?;

        // let sub_reward_token_pool_option :Option<AccountInfo>;
        // let sub_reward_destination_option :Option<AccountInfo>;
        if with_subrewards != fill_sub_rewards_accounts.is_some() {
            return Err(ProgramError::InvalidArgument);
        };

        let sub_reward = if with_subrewards {
            let sub_reward_token_pool = AccountLoader::next_unchecked(account_info_iter)?;

            Some((
                sub_reward_token_pool,
                fill_sub_rewards_accounts.unwrap().reward_transit_info,
            ))
        } else {
            None
        };

        Ok(PortFinanceClaimer {
            stake_account,
            staking_pool,
            staking_pool_authority,
            reward_token_pool,
            clock,
            sub_reward,
        })
    }
}

impl<'a, 'b> RewardClaimer<'b> for PortFinanceClaimer<'a, 'b> {
    ///
    fn claim_reward(
        &self,
        staking_program_id: &Pubkey,
        reward_transit_token_account: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        port_finance::claim_reward(
            staking_program_id,
            authority,
            self.stake_account.clone(),
            self.staking_pool.clone(),
            self.staking_pool_authority.clone(),
            self.reward_token_pool.clone(),
            reward_transit_token_account,
            self.sub_reward,
            self.clock.clone(),
            signers_seeds,
        )?;
        Ok(())
    }
}
