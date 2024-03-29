use super::CollateralStorage;
use everlend_utils::{cpi::quarry, AccountLoader};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;
use std::{iter::Enumerate, slice::Iter};

///
#[derive(Clone)]
pub struct Quarry<'a, 'b> {
    quarry_mining_program_id: Pubkey,
    miner: &'a AccountInfo<'b>,
    rewarder: &'a AccountInfo<'b>,
    quarry: &'a AccountInfo<'b>,
    miner_vault: &'a AccountInfo<'b>,
}

impl<'a, 'b> Quarry<'a, 'b> {
    ///
    pub fn init(
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        depositor_authority_pubkey: &Pubkey,
        token_mint: &Pubkey,
        rewarder: &Pubkey,
    ) -> Result<Quarry<'a, 'b>, ProgramError> {
        let quarry_mining_program_id_info =
            AccountLoader::next_with_key(account_info_iter, &quarry::staking_program_id())?;
        let rewarder = AccountLoader::next_with_key(account_info_iter, rewarder)?;

        let quarry_info = {
            let (quarry, _) = quarry::find_quarry_program_address(
                quarry_mining_program_id_info.key,
                rewarder.key,
                token_mint,
            );

            AccountLoader::next_with_key(account_info_iter, &quarry)
        }?;

        let miner = {
            let (miner_pubkey, _) = quarry::find_miner_program_address(
                quarry_mining_program_id_info.key,
                quarry_info.key,
                depositor_authority_pubkey,
            );

            AccountLoader::next_with_key(account_info_iter, &miner_pubkey)
        }?;

        let miner_vault = {
            let miner_vault = get_associated_token_address(miner.key, token_mint);
            AccountLoader::next_with_key(account_info_iter, &miner_vault)
        }?;

        Ok(Quarry {
            quarry_mining_program_id: *quarry_mining_program_id_info.key,
            miner,
            rewarder,
            quarry: quarry_info,
            miner_vault,
        })
    }
}

impl<'a, 'b> CollateralStorage<'b> for Quarry<'a, 'b> {
    /// Deposit collateral tokens
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        quarry::stake_tokens(
            &self.quarry_mining_program_id,
            authority.clone(),
            self.miner.clone(),
            self.quarry.clone(),
            self.miner_vault.clone(),
            collateral_transit.clone(),
            self.rewarder.clone(),
            collateral_amount,
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
        quarry::withdraw_tokens(
            &self.quarry_mining_program_id,
            authority.clone(),
            self.miner.clone(),
            self.quarry.clone(),
            self.miner_vault.clone(),
            collateral_transit.clone(),
            self.rewarder.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        Ok(())
    }
}
