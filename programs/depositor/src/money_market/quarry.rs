use super::{CollateralStorage};
use everlend_utils::{assert_account_key, cpi::quarry};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_associated_token_account::get_associated_token_address;
use std::slice::Iter;

///
#[derive(Clone)]
pub struct Quarry<'a> {
    quarry_mining_program_id: Pubkey,
    miner: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
}

impl<'a, 'b> Quarry<'a> {
    ///
    pub fn init(
        account_info_iter: &'b mut Iter<AccountInfo<'a>>,
        depositor_authority_pubkey: &Pubkey,
        token_mint: &Pubkey,
        rewarder: &Pubkey,
    ) -> Result<Quarry<'a>, ProgramError> {
        let quarry_mining_program_id_info = next_account_info(account_info_iter)?;
        assert_account_key(quarry_mining_program_id_info, &quarry::staking_program_id())?;

        let rewarder_info = next_account_info(account_info_iter)?;
        assert_account_key(rewarder_info, rewarder)?;

        let quarry_info = next_account_info(account_info_iter)?;
        let (quarry, _) = quarry::find_quarry_program_address(
            quarry_mining_program_id_info.key,
            rewarder_info.key,
            token_mint,
        );
        assert_account_key(quarry_info, &quarry)?;

        let miner_info = next_account_info(account_info_iter)?;
        let (miner_pubkey, _) = quarry::find_miner_program_address(
            quarry_mining_program_id_info.key,
            &quarry,
            depositor_authority_pubkey,
        );
        assert_account_key(miner_info, &miner_pubkey)?;

        let miner_vault_info = next_account_info(account_info_iter)?;
        let miner_vault = get_associated_token_address(&miner_pubkey, token_mint);
        assert_account_key(miner_vault_info, &miner_vault)?;

        Ok(Quarry {
            quarry_mining_program_id: *quarry_mining_program_id_info.key,
            miner: miner_info.clone(),
            rewarder: rewarder_info.clone(),
            quarry: quarry_info.clone(),
            miner_vault: miner_vault_info.clone(),
        })
    }
}

impl<'a> CollateralStorage<'a> for Quarry<'a> {
    /// Deposit collateral tokens
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
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
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
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
