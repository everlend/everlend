use crate::claimer::RewardClaimer;
use crate::state::MiningType;
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
    mint_wrapper: &'a AccountInfo<'b>,
    mint_wrapper_program: &'a AccountInfo<'b>,
    minter: &'a AccountInfo<'b>,
    rewards_token_mint: &'a AccountInfo<'b>,
    rewards_token_account: &'a AccountInfo<'b>,
    claim_fee_token_account: &'a AccountInfo<'b>,
    stake_token_account: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    merge_miner: &'a AccountInfo<'b>,
    rewarder: &'a AccountInfo<'b>,
    quarry: &'a AccountInfo<'b>,
    miner: &'a AccountInfo<'b>,
    miner_vault: &'a AccountInfo<'b>,
}

impl<'a, 'b> QuarryMergeClaimer<'a, 'b> {
    ///
    pub fn init(
        depositor_authority: &Pubkey,
        staking_program_id: &Pubkey,
        collateral_mint: &Pubkey,
        internal_mining_type: MiningType,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<QuarryMergeClaimer<'a, 'b>, ProgramError> {
        if !staking_program_id.eq(&quarry_merge::staking_program_id()) {
            return Err(ProgramError::InvalidArgument);
        }

        let (pool_pubkey, rewarder_pubkey) = match internal_mining_type {
            MiningType::QuarryMerge { pool, rewarder } => (pool, rewarder),
            _ => return Err(EverlendError::MiningNotInitialized.into()),
        };

        let mint_wrapper = AccountLoader::next_unchecked(account_info_iter)?;
        let mint_wrapper_program = AccountLoader::next_unchecked(account_info_iter)?;
        let minter = AccountLoader::next_with_owner(account_info_iter, &mint_wrapper_program.key)?;
        let rewards_token_mint = AccountLoader::next_unchecked(account_info_iter)?;

        let rewards_token_account =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let claim_fee_token_account =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let stake_token_account =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let pool = AccountLoader::next_with_key(account_info_iter, &pool_pubkey)?;

        let merge_miner = {
            let (merge_miner_pubkey, _) = quarry_merge::find_merge_miner_program_address(
                &quarry_merge::staking_program_id(),
                pool.key,
                depositor_authority,
            );
            AccountLoader::next_with_key(account_info_iter, &merge_miner_pubkey)
        }?;

        let rewarder = AccountLoader::next_with_key(account_info_iter, &rewarder_pubkey)?;
        let quarry = {
            let (quarry, _) = quarry::find_quarry_program_address(
                &quarry::staking_program_id(),
                rewarder.key,
                collateral_mint,
            );

            AccountLoader::next_with_key(account_info_iter, &quarry)
        }?;
        let miner = {
            let (miner_pubkey, _) = quarry::find_miner_program_address(
                &quarry::staking_program_id(),
                quarry.key,
                merge_miner.key,
            );
            AccountLoader::next_with_key(account_info_iter, &miner_pubkey)
        }?;
        let miner_vault = {
            let miner_vault = get_associated_token_address(miner.key, collateral_mint);
            AccountLoader::next_with_key(account_info_iter, &miner_vault)
        }?;

        Ok(QuarryMergeClaimer {
            mint_wrapper,
            mint_wrapper_program,
            minter,
            rewards_token_mint,
            rewards_token_account,
            claim_fee_token_account,
            stake_token_account,
            pool,
            merge_miner,
            rewarder,
            quarry,
            miner,
            miner_vault,
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
        quarry_merge::claim_rewards(
            staking_program_id,
            self.mint_wrapper.clone(),
            self.mint_wrapper_program.clone(),
            self.minter.clone(),
            self.rewards_token_mint.clone(),
            self.rewards_token_account.clone(),
            self.claim_fee_token_account.clone(),
            self.stake_token_account.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.rewarder.clone(),
            self.quarry.clone(),
            self.miner.clone(),
            self.miner_vault.clone(),
        )?;

        let amount =
            Account::unpack_from_slice(self.rewards_token_account.data.borrow().as_ref())?.amount;

        quarry_merge::withdraw_tokens(
            staking_program_id,
            authority,
            self.pool.clone(),
            self.merge_miner.clone(),
            self.rewards_token_mint.clone(),
            self.rewards_token_account.clone(),
            reward_transit_token_account,
            amount,
            signers_seeds,
        )?;

        Ok(())
    }
}
