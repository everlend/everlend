use crate::claimer::RewardClaimer;
use crate::state::MiningType;
use crate::utils::FillRewardAccounts;
use everlend_utils::cpi::{quarry, quarry_merge};
use everlend_utils::{AccountLoader, EverlendError};
use solana_program::program_pack::Pack;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

/// Container
#[derive(Clone)]
pub struct QuarryMergeClaimer<'a, 'b> {
    mint_wrapper_primary: &'a AccountInfo<'b>,
    mint_wrapper_replica: &'a AccountInfo<'b>,
    mint_wrapper_program: &'a AccountInfo<'b>,
    minter_primary: &'a AccountInfo<'b>,
    minter_replica: &'a AccountInfo<'b>,
    rewards_token_mint_primary: &'a AccountInfo<'b>,
    rewards_token_mint_replica: &'a AccountInfo<'b>,
    rewards_token_account_primary: &'a AccountInfo<'b>,
    rewards_token_account_replica: &'a AccountInfo<'b>,
    claim_fee_token_account_primary: &'a AccountInfo<'b>,
    claim_fee_token_account_replica: &'a AccountInfo<'b>,
    stake_token_account_primary: &'a AccountInfo<'b>,
    stake_token_account_replica: &'a AccountInfo<'b>,
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
    sub_reward: &'a AccountInfo<'b>,
}

impl<'a, 'b> QuarryMergeClaimer<'a, 'b> {
    ///
    pub fn init(
        depositor_authority: &Pubkey,
        staking_program_id: &Pubkey,
        collateral_mint: &Pubkey,
        internal_mining_type: MiningType,
        fill_sub_rewards_accounts: Option<FillRewardAccounts<'a, 'b>>,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<QuarryMergeClaimer<'a, 'b>, ProgramError> {
        if !staking_program_id.eq(&quarry_merge::staking_program_id()) {
            return Err(ProgramError::InvalidArgument);
        }

        let (rewarder_primary, rewarder_replica) = match internal_mining_type {
            MiningType::QuarryMerge {
                rewarder_primary,
                rewarder_replica,
            } => (rewarder_primary, rewarder_replica),
            _ => return Err(EverlendError::MiningNotInitialized.into()),
        };

        let mint_wrapper_primary = AccountLoader::next_unchecked(account_info_iter)?;
        let mint_wrapper_replica = AccountLoader::next_unchecked(account_info_iter)?;
        let mint_wrapper_program =
            AccountLoader::next_with_key(account_info_iter, &quarry::mine_wrapper_program_id())?;
        let minter_primary = {
            let (merge_miner_pubkey, _) = quarry::find_minter_program_address(
                &mint_wrapper_program.key,
                mint_wrapper_primary.key,
                &rewarder_primary,
            );
            AccountLoader::next_with_key(account_info_iter, &merge_miner_pubkey)
        }?;
        let minter_replica = {
            let (merge_miner_pubkey, _) = quarry::find_minter_program_address(
                &mint_wrapper_program.key,
                mint_wrapper_replica.key,
                &rewarder_replica,
            );
            AccountLoader::next_with_key(account_info_iter, &merge_miner_pubkey)
        }?;
        let rewards_token_mint_primary = AccountLoader::next_unchecked(account_info_iter)?;
        let rewards_token_mint_replica = AccountLoader::next_unchecked(account_info_iter)?;

        let rewards_token_account_primary =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let rewards_token_account_replica =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let claim_fee_token_account_primary =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let claim_fee_token_account_replica =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let stake_token_account_primary =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let stake_token_account_replica =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let pool = {
            let (pool_pubkey, _) = quarry_merge::find_pool_program_address(
                &quarry_merge::staking_program_id(),
                collateral_mint,
            );
            AccountLoader::next_with_key(account_info_iter, &pool_pubkey)
        }?;
        let merge_miner = {
            let (merge_miner_pubkey, _) = quarry_merge::find_merge_miner_program_address(
                &quarry_merge::staking_program_id(),
                pool.key,
                depositor_authority,
            );
            AccountLoader::next_with_key(account_info_iter, &merge_miner_pubkey)
        }?;

        let rewarder_primary = AccountLoader::next_with_key(account_info_iter, &rewarder_primary)?;
        let rewarder_replica = AccountLoader::next_with_key(account_info_iter, &rewarder_replica)?;
        let quarry_primary = {
            let (quarry, _) = quarry::find_quarry_program_address(
                &quarry::staking_program_id(),
                rewarder_primary.key,
                collateral_mint,
            );

            AccountLoader::next_with_key(account_info_iter, &quarry)
        }?;
        let (replica_mint, _) = quarry_merge::find_replica_mint_program_address(
            &quarry_merge::staking_program_id(),
            pool.key,
        );
        let quarry_replica = {
            let (quarry, _) = quarry::find_quarry_program_address(
                &quarry::staking_program_id(),
                rewarder_replica.key,
                &replica_mint,
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
            let miner_vault = get_associated_token_address(miner_primary.key, collateral_mint);
            AccountLoader::next_with_key(account_info_iter, &miner_vault)
        }?;
        let miner_vault_replica = {
            let miner_vault = get_associated_token_address(miner_replica.key, &replica_mint);
            AccountLoader::next_with_key(account_info_iter, &miner_vault)
        }?;

        if fill_sub_rewards_accounts.is_none() {
            return Err(ProgramError::InvalidArgument);
        }
        let sub_reward = fill_sub_rewards_accounts
            .as_ref()
            .unwrap()
            .reward_transit_info;

        Ok(QuarryMergeClaimer {
            mint_wrapper_primary,
            mint_wrapper_replica,
            mint_wrapper_program,
            minter_primary,
            minter_replica,
            rewards_token_mint_primary,
            rewards_token_mint_replica,
            rewards_token_account_primary,
            rewards_token_account_replica,
            claim_fee_token_account_primary,
            claim_fee_token_account_replica,
            stake_token_account_primary,
            stake_token_account_replica,
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
            sub_reward,
        })
    }
}

impl<'a, 'b> RewardClaimer<'b> for QuarryMergeClaimer<'a, 'b> {
    ///
    fn claim_reward(
        &self,
        staking_program_id: &Pubkey,
        reward_transit_token_account: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        let amount_replica =
            Account::unpack_from_slice(self.rewards_token_account_replica.data.borrow().as_ref())?
                .amount;
        let amount_primary =
            Account::unpack_from_slice(self.rewards_token_account_primary.data.borrow().as_ref())?
                .amount;

        if amount_primary != 0 {
            quarry_merge::claim_rewards(
                staking_program_id,
                self.mint_wrapper_primary.clone(),
                self.mint_wrapper_program.clone(),
                self.minter_primary.clone(),
                self.rewards_token_mint_primary.clone(),
                self.rewards_token_account_primary.clone(),
                self.claim_fee_token_account_primary.clone(),
                self.stake_token_account_primary.clone(),
                self.pool.clone(),
                self.merge_miner.clone(),
                self.rewarder_primary.clone(),
                self.quarry_primary.clone(),
                self.miner_primary.clone(),
                self.miner_vault_primary.clone(),
            )?;

            quarry_merge::withdraw_tokens(
                staking_program_id,
                authority.clone(),
                self.pool.clone(),
                self.merge_miner.clone(),
                self.rewards_token_mint_primary.clone(),
                self.rewards_token_account_primary.clone(),
                reward_transit_token_account,
                amount_primary,
                signers_seeds,
            )?;
        }

        if amount_replica != 0 {
            quarry_merge::claim_rewards(
                staking_program_id,
                self.mint_wrapper_replica.clone(),
                self.mint_wrapper_program.clone(),
                self.minter_replica.clone(),
                self.rewards_token_mint_replica.clone(),
                self.rewards_token_account_replica.clone(),
                self.claim_fee_token_account_replica.clone(),
                self.stake_token_account_replica.clone(),
                self.pool.clone(),
                self.merge_miner.clone(),
                self.rewarder_replica.clone(),
                self.quarry_replica.clone(),
                self.miner_replica.clone(),
                self.miner_vault_replica.clone(),
            )?;

            quarry_merge::withdraw_tokens(
                staking_program_id,
                authority,
                self.pool.clone(),
                self.merge_miner.clone(),
                self.rewards_token_mint_replica.clone(),
                self.rewards_token_account_replica.clone(),
                self.sub_reward.clone(),
                amount_replica,
                signers_seeds,
            )?;
        }

        Ok(())
    }
}
