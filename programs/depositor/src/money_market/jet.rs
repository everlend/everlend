use crate::money_market::MoneyMarket;
use everlend_utils::{cpi::jet, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

///
pub struct Jet<'a, 'b> {
    money_market_program_id: Pubkey,
    margin_pool: &'a AccountInfo<'b>,
    vault: &'a AccountInfo<'b>,
}

impl<'a, 'b> Jet<'a, 'b> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<Jet<'a, 'b>, ProgramError> {
        let margin_pool_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let vault_info = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        Ok(Jet {
            money_market_program_id,
            margin_pool: margin_pool_info,
            vault: vault_info,
        })
    }
}

impl<'a, 'b> MoneyMarket<'b> for Jet<'a, 'b> {
    ///
    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_liquidity: AccountInfo<'b>,
        destination_collateral: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        jet::deposit(
            &self.money_market_program_id,
            self.margin_pool.clone(),
            self.vault.clone(),
            collateral_mint,
            authority,
            source_liquidity,
            destination_collateral.clone(),
            liquidity_amount,
            signers_seeds,
        )?;

        let collateral_amount =
            Account::unpack_from_slice(&destination_collateral.data.borrow())?.amount;

        Ok(collateral_amount)
    }

    ///
    fn money_market_redeem(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_collateral: AccountInfo<'b>,
        destination_liquidity: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        jet::redeem(
            &self.money_market_program_id,
            self.margin_pool.clone(),
            self.vault.clone(),
            collateral_mint,
            authority,
            source_collateral,
            destination_liquidity,
            collateral_amount,
            signers_seeds,
        )
    }

    ///
    fn money_market_deposit_and_deposit_mining(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _source_liquidity: AccountInfo<'b>,
        _collateral_transit: AccountInfo<'b>,
        _authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        _liquidity_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        Err(EverlendError::MiningNotImplemented.into())
    }

    ///
    fn money_market_redeem_and_withdraw_mining(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _collateral_transit: AccountInfo<'b>,
        _liquidity_destination: AccountInfo<'b>,
        _authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        _collateral_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        Err(EverlendError::MiningNotImplemented.into())
    }

    fn is_deposit_disabled(&self) -> Result<bool, ProgramError> {
        jet::is_deposit_disabled(self.margin_pool.clone())
    }
}
