use crate::claimer::RewardClaimer;
use crate::find_transit_program_address;
use crate::state::MiningType;
use everlend_utils::cpi::quarry;
use everlend_utils::{AccountLoader, EverlendError};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use std::{iter::Enumerate, slice::Iter};

/// Container
#[derive(Clone)]
pub struct QuarryClaimer<'a, 'b> {
    mint_wrapper: &'a AccountInfo<'b>,
    mint_wrapper_program: &'a AccountInfo<'b>,
    minter: &'a AccountInfo<'b>,
    rewards_token_mint: &'a AccountInfo<'b>,
    rewards_token_account: &'a AccountInfo<'b>,
    rewards_fee_account: &'a AccountInfo<'b>,
    quarry_rewarder: &'a AccountInfo<'b>,
    quarry_info: &'a AccountInfo<'b>,
    miner: &'a AccountInfo<'b>,
    redeemer_program_id_info: &'a AccountInfo<'b>,
    redeemer_info: &'a AccountInfo<'b>,
    redemption_vault_info: &'a AccountInfo<'b>,
}

impl<'a, 'b> QuarryClaimer<'a, 'b> {
    ///
    pub fn init(
        program_id: &Pubkey,
        depositor: &Pubkey,
        depositor_authority: &Pubkey,
        collateral_mint: &Pubkey,
        staking_program_id: &Pubkey,
        internal_mining_type: MiningType,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<QuarryClaimer<'a, 'b>, ProgramError> {
        if !staking_program_id.eq(&quarry::staking_program_id()) {
            return Err(ProgramError::InvalidArgument);
        }

        // Parse mining  accounts if presented
        let rewarder_pubkey = match internal_mining_type {
            MiningType::Quarry { rewarder } => rewarder,
            _ => return Err(EverlendError::MiningNotInitialized.into()),
        };

        // TODO add checks for accounts
        let mint_wrapper = AccountLoader::next_unchecked(account_info_iter)?;
        let mint_wrapper_program = AccountLoader::next_unchecked(account_info_iter)?;
        let minter = AccountLoader::next_unchecked(account_info_iter)?;
        // IOU token mint
        let rewards_token_mint = AccountLoader::next_unchecked(account_info_iter)?;

        let rewards_token_account = {
            let (reward_token_account_pubkey, _) = find_transit_program_address(
                program_id,
                depositor,
                rewards_token_mint.key,
                "lm_reward",
            );

            AccountLoader::next_with_key(account_info_iter, &reward_token_account_pubkey)?
        };

        let rewards_fee_account =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let quarry_rewarder = AccountLoader::next_with_key(account_info_iter, &rewarder_pubkey)?;

        let quarry_info = {
            let (quarry, _) = quarry::find_quarry_program_address(
                staking_program_id,
                quarry_rewarder.key,
                collateral_mint,
            );

            AccountLoader::next_with_key(account_info_iter, &quarry)
        }?;

        let miner = {
            let (miner_pubkey, _) = quarry::find_miner_program_address(
                &quarry::staking_program_id(),
                quarry_info.key,
                depositor_authority,
            );

            AccountLoader::next_with_key(account_info_iter, &miner_pubkey)
        }?;

        let redeemer_program_id_info = AccountLoader::next_unchecked(account_info_iter)?;
        let redeemer_info = AccountLoader::next_unchecked(account_info_iter)?;
        let redemption_vault_info =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        Ok(QuarryClaimer {
            mint_wrapper,
            mint_wrapper_program,
            minter,
            rewards_token_mint,
            rewards_token_account,
            rewards_fee_account,
            quarry_rewarder,
            quarry_info,
            miner,
            redeemer_program_id_info,
            redeemer_info,
            redemption_vault_info,
        })
    }
}

impl<'a, 'b> RewardClaimer<'b> for QuarryClaimer<'a, 'b> {
    ///
    fn claim_reward(
        &self,
        staking_program_id: &Pubkey,
        reward_transit_token_account: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        quarry::claim_rewards(
            staking_program_id,
            self.mint_wrapper.clone(),
            self.mint_wrapper_program.clone(),
            self.minter.clone(),
            self.rewards_token_mint.clone(),
            self.rewards_token_account.clone(),
            self.rewards_fee_account.clone(),
            authority.clone(),
            self.miner.clone(),
            self.quarry_info.clone(),
            self.quarry_rewarder.clone(),
            signers_seeds,
        )?;

        quarry::redeem_all_tokens(
            self.redeemer_program_id_info.key,
            self.redeemer_info.clone(),
            self.rewards_token_mint.clone(),
            self.rewards_token_account.clone(),
            self.redemption_vault_info.clone(),
            reward_transit_token_account,
            authority,
            signers_seeds,
        )?;

        Ok(())
    }
}
