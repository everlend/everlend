use crate::money_market::MoneyMarket;
use everlend_utils::{cpi::tulip, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

///
pub struct Tulip<'a, 'b> {
    money_market_program_id: Pubkey,
    reserve: &'a AccountInfo<'b>,
    reserve_liquidity_supply: &'a AccountInfo<'b>,
    lending_market: &'a AccountInfo<'b>,
    lending_market_authority: &'a AccountInfo<'b>,
    reserve_liquidity_oracle: &'a AccountInfo<'b>,
}

impl<'a, 'b> Tulip<'a, 'b> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<Tulip<'a, 'b>, ProgramError> {
        let reserve_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let reserve_liquidity_supply_info =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let lending_market_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let lending_market_authority_info = AccountLoader::next_unchecked(account_info_iter)?;
        let reserve_liquidity_oracle_info = AccountLoader::next_unchecked(account_info_iter)?;

        Ok(Tulip {
            money_market_program_id,
            reserve: reserve_info,
            reserve_liquidity_supply: reserve_liquidity_supply_info,
            lending_market: lending_market_info,
            lending_market_authority: lending_market_authority_info,
            reserve_liquidity_oracle: reserve_liquidity_oracle_info,
        })
    }
}

impl<'a, 'b> MoneyMarket<'b> for Tulip<'a, 'b> {
    ///
    fn is_collateral_return(&self) -> bool {
        true
    }

    ///
    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_liquidity: AccountInfo<'b>,
        destination_collateral: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        tulip::deposit(
            &self.money_market_program_id,
            source_liquidity,
            destination_collateral.clone(),
            self.reserve.clone(),
            collateral_mint,
            self.reserve_liquidity_supply.clone(),
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            authority,
            clock,
            amount,
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
        clock: AccountInfo<'b>,
        amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        tulip::redeem(
            &self.money_market_program_id,
            source_collateral,
            destination_liquidity,
            self.reserve.clone(),
            collateral_mint,
            self.reserve_liquidity_supply.clone(),
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            authority,
            clock,
            amount,
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
        _amount: u64,
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
        _amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        Err(EverlendError::MiningNotImplemented.into())
    }

    fn is_income(
        &self,
        collateral_amount: u64,
        expected_liquidity_amount: u64,
    ) -> Result<bool, ProgramError> {
        let real_liquidity_amount =
            tulip::get_real_liquidity_amount(self.reserve.clone(), collateral_amount)?;

        Ok(real_liquidity_amount > expected_liquidity_amount)
    }

    fn refresh_reserve(&self, clock: AccountInfo<'b>) -> Result<(), ProgramError> {
        tulip::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
            clock.clone(),
        )
    }

    fn is_deposit_disabled(&self) -> Result<bool, ProgramError> {
        // Not presented
        Ok(false)
    }
}
