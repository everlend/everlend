use crate::claimer::RewardClaimer;
use everlend_utils::{cpi, AccountLoader, EverlendError};
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use std::iter::Enumerate;
use std::slice::Iter;

/// Container
#[derive(Clone)]
pub struct FraktClaimer<'a, 'b> {
    liquidity_pool: &'a AccountInfo<'b>,
    deposit_account: &'a AccountInfo<'b>,
    liquidity_owner: &'a AccountInfo<'b>,
    admin: &'a AccountInfo<'b>,
    deposit_bump: u8,
}

impl<'a, 'b> FraktClaimer<'a, 'b> {
    ///
    pub fn init(
        staking_program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        authority: &Pubkey,
    ) -> Result<FraktClaimer<'a, 'b>, ProgramError> {
        let liquidity_pool = AccountLoader::next_with_owner(account_info_iter, staking_program_id)?;
        let (deposit_account, deposit_bump) = {
            let (deposit_account_pubkey, deposit_bump) =
                cpi::frakt::find_deposit_address(staking_program_id, liquidity_pool.key, authority);
            (
                AccountLoader::next_with_key(account_info_iter, &deposit_account_pubkey)?,
                deposit_bump,
            )
        };
        let liquidity_owner = {
            let (liquidity_owner_pubkey, _) =
                cpi::frakt::find_owner_address(staking_program_id, liquidity_pool.key);
            AccountLoader::next_with_key(account_info_iter, &liquidity_owner_pubkey)?
        };
        let admin = AccountLoader::next_unchecked(account_info_iter)?;
        let _system = AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        Ok(FraktClaimer {
            liquidity_pool,
            deposit_account,
            liquidity_owner,
            admin,
            deposit_bump,
        })
    }
}

impl<'a, 'b> RewardClaimer<'b> for FraktClaimer<'a, 'b> {
    fn claim_reward(
        &self,
        staking_program_id: &Pubkey,
        reward_transit_token_account: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        let starting_lamports = authority.lamports();

        cpi::frakt::claim_rewards(
            staking_program_id,
            self.liquidity_pool.clone(),
            self.deposit_account.clone(),
            authority.clone(),
            self.liquidity_owner.clone(),
            self.admin.clone(),
            self.deposit_bump,
            &signers_seeds,
        )?;

        let rewards = starting_lamports
            .checked_sub(authority.lamports())
            .ok_or(EverlendError::MathOverflow)?;

        cpi::system::transfer(
            authority.clone(),
            reward_transit_token_account.clone(),
            rewards,
            signers_seeds,
        )?;

        cpi::spl_token::sync_native(reward_transit_token_account)?;

        Ok(())
    }
}
