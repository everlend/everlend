use super::CollateralStorage;

use everlend_utils::cpi::quarry_merge;
use everlend_utils::{
    cpi::{quarry, spl_token},
    AccountLoader,
};
use solana_program::program_pack::Pack;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;
use ::spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

///
#[derive(Clone)]
pub struct QuarryMerge<'a, 'b> {
    quarry_merge_mining_program_id: Pubkey,
    mm_primary_token_account: &'a AccountInfo<'b>,
    primary_token_mint: &'a AccountInfo<'b>,
    replica_mint: &'a AccountInfo<'b>,
    replica_mint_token_account: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    merge_miner: &'a AccountInfo<'b>,
    rewarder_primary: &'a AccountInfo<'b>,
    rewarder_replica: &'a AccountInfo<'b>,
    quarry_primary: &'a AccountInfo<'b>,
    quarry_replica: &'a AccountInfo<'b>,
    miner_primary: &'a AccountInfo<'b>,
    miner_replica: &'a AccountInfo<'b>,
    miner_vault_primary: &'a AccountInfo<'b>,
    miner_vault_replica: &'a AccountInfo<'b>,
}

impl<'a, 'b> QuarryMerge<'a, 'b> {
    ///
    pub fn init(
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        depositor_authority_pubkey: &Pubkey,
        token_mint: &'a AccountInfo<'b>,
        rewarder_primary: &Pubkey,
        rewarder_replica: &Pubkey,
    ) -> Result<QuarryMerge<'a, 'b>, ProgramError> {
        let quarry_merge_mining_program_id_info =
            AccountLoader::next_with_key(account_info_iter, &quarry_merge::staking_program_id())?;
        let mm_primary_token_account =
            AccountLoader::next_with_owner(account_info_iter, &::spl_token::id())?;
        let replica_mint_token_account =
            AccountLoader::next_with_owner(account_info_iter, &::spl_token::id())?;
        let pool = {
            let (pool_pubkey, _) = quarry_merge::find_pool_program_address(
                &quarry_merge::staking_program_id(),
                token_mint.key,
            );
            AccountLoader::next_with_key(account_info_iter, &pool_pubkey)
        }?;
        let replica_mint = {
            let (replica_mint_pubkey, _) = quarry_merge::find_replica_mint_program_address(
                &quarry_merge::staking_program_id(),
                &pool.key,
            );
            AccountLoader::next_with_key(account_info_iter, &replica_mint_pubkey)
        }?;

        let merge_miner = {
            let (merge_miner_pubkey, _) = quarry_merge::find_merge_miner_program_address(
                &quarry_merge::staking_program_id(),
                pool.key,
                depositor_authority_pubkey,
            );
            AccountLoader::next_with_key(account_info_iter, &merge_miner_pubkey)
        }?;
        let rewarder_primary = AccountLoader::next_with_key(account_info_iter, &rewarder_primary)?;
        let rewarder_replica = AccountLoader::next_with_key(account_info_iter, &rewarder_replica)?;
        let quarry_primary = {
            let (quarry, _) = quarry::find_quarry_program_address(
                &quarry::staking_program_id(),
                rewarder_primary.key,
                token_mint.key,
            );
            AccountLoader::next_with_key(account_info_iter, &quarry)
        }?;
        let quarry_replica = {
            let (quarry, _) = quarry::find_quarry_program_address(
                &quarry::staking_program_id(),
                rewarder_replica.key,
                replica_mint.key,
            );
            AccountLoader::next_with_key(account_info_iter, &quarry)
        }?;
        let miner_primary = {
            let (miner_pubkey, _) = quarry::find_miner_program_address(
                &quarry::staking_program_id(),
                quarry_primary.key,
                merge_miner.key,
            );
            AccountLoader::next_with_key(account_info_iter, &miner_pubkey)
        }?;
        let miner_replica = {
            let (miner_pubkey, _) = quarry::find_miner_program_address(
                &quarry::staking_program_id(),
                quarry_replica.key,
                merge_miner.key,
            );
            AccountLoader::next_with_key(account_info_iter, &miner_pubkey)
        }?;
        let miner_vault_primary = {
            let miner_vault = get_associated_token_address(miner_primary.key, token_mint.key);
            AccountLoader::next_with_key(account_info_iter, &miner_vault)
        }?;
        let miner_vault_replica = {
            let miner_vault = get_associated_token_address(miner_replica.key, replica_mint.key);
            AccountLoader::next_with_key(account_info_iter, &miner_vault)
        }?;

        Ok(QuarryMerge {
            quarry_merge_mining_program_id: *quarry_merge_mining_program_id_info.key,
            mm_primary_token_account,
            primary_token_mint: token_mint,
            replica_mint,
            replica_mint_token_account,
            pool,
            merge_miner,
            rewarder_primary,
            rewarder_replica,
            quarry_primary,
            quarry_replica,
            miner_primary,
            miner_replica,
            miner_vault_primary,
            miner_vault_replica,
        })
    }
}

impl<'a, 'b> CollateralStorage<'b> for QuarryMerge<'a, 'b> {
    /// Deposit collateral tokens
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        spl_token::transfer(
            collateral_transit.clone(),
            self.mm_primary_token_account.clone(),
            authority.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        quarry_merge::stake_primary(
            &self.quarry_merge_mining_program_id,
            authority.clone(),
            self.mm_primary_token_account.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.rewarder_primary.clone(),
            self.quarry_primary.clone(),
            self.miner_primary.clone(),
            self.miner_vault_primary.clone(),
            signers_seeds,
        )?;

        quarry_merge::stake_replica(
            &self.quarry_merge_mining_program_id,
            authority.clone(),
            self.replica_mint.clone(),
            self.replica_mint_token_account.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.rewarder_replica.clone(),
            self.quarry_replica.clone(),
            self.miner_replica.clone(),
            self.miner_vault_replica.clone(),
            signers_seeds,
        )?;

        Ok(())
    }
    /// Withdraw collateral tokens
    fn withdraw_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        quarry_merge::unstake_replica(
            &self.quarry_merge_mining_program_id,
            authority.clone(),
            self.replica_mint.clone(),
            self.replica_mint_token_account.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.rewarder_replica.clone(),
            self.quarry_replica.clone(),
            self.miner_replica.clone(),
            self.miner_vault_replica.clone(),
            signers_seeds,
        )?;

        quarry_merge::unstake_primary(
            &self.quarry_merge_mining_program_id,
            authority.clone(),
            self.mm_primary_token_account.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.rewarder_primary.clone(),
            self.quarry_primary.clone(),
            self.miner_primary.clone(),
            self.miner_vault_primary.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        quarry_merge::withdraw_tokens(
            &self.quarry_merge_mining_program_id,
            authority.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.primary_token_mint.clone(),
            self.mm_primary_token_account.clone(),
            collateral_transit.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        if Account::unpack_from_slice(self.mm_primary_token_account.data.borrow().as_ref())?.amount
            != 0
        {
            quarry_merge::stake_replica(
                &self.quarry_merge_mining_program_id,
                authority.clone(),
                self.replica_mint.clone(),
                self.replica_mint_token_account.clone(),
                self.pool.clone(),
                self.merge_miner.clone(),
                self.rewarder_replica.clone(),
                self.quarry_replica.clone(),
                self.miner_replica.clone(),
                self.miner_vault_replica.clone(),
                signers_seeds,
            )?;
        }

        Ok(())
    }
}
