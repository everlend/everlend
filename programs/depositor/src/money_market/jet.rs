use crate::money_market::{MoneyMarket};
use everlend_utils::{cpi::jet, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

///
pub struct Jet<'a> {
    money_market_program_id: Pubkey,
    margin_pool: AccountInfo<'a>,
    vault: AccountInfo<'a>,
}

impl<'a, 'b> Jet<'a> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        account_info_iter: &'b mut Enumerate<Iter<'_, AccountInfo<'a>>>,
    ) -> Result<Jet<'a>, ProgramError> {
        let margin_pool_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let vault_info =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        Ok(Jet {
            money_market_program_id,
            margin_pool: margin_pool_info.clone(),
            vault: vault_info.clone(),
        })
    }
}

impl<'a> MoneyMarket<'a> for Jet<'a> {
    ///
    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'a>,
        source_liquidity: AccountInfo<'a>,
        destination_collateral: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
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
        collateral_mint: AccountInfo<'a>,
        source_collateral: AccountInfo<'a>,
        destination_liquidity: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
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
        _collateral_mint: AccountInfo<'a>,
        _source_liquidity: AccountInfo<'a>,
        _collateral_transit: AccountInfo<'a>,
        _authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
        _liquidity_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> { Err(EverlendError::MiningNotImplemented.into()) }

    ///
    fn money_market_redeem_and_withdraw_mining(
        &self,
        _collateral_mint: AccountInfo<'a>,
        _collateral_transit: AccountInfo<'a>,
        _liquidity_destination: AccountInfo<'a>,
        _authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
        _collateral_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> { Err(EverlendError::MiningNotImplemented.into()) }
}