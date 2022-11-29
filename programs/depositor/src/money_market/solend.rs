use super::MoneyMarket;
use crate::money_market::assert_valid_money_market;
use everlend_utils::{cpi::solend, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

///
pub struct Solend<'a, 'b> {
    money_market_program_id: Pubkey,
    reserve: &'a AccountInfo<'b>,
    reserve_liquidity_supply: &'a AccountInfo<'b>,
    lending_market: &'a AccountInfo<'b>,
    lending_market_authority: &'a AccountInfo<'b>,
    reserve_liquidity_pyth_oracle: &'a AccountInfo<'b>,
    reserve_liquidity_switchboard_oracle: &'a AccountInfo<'b>,
}

impl<'a, 'b> Solend<'a, 'b> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        money_market: everlend_registry::state::MoneyMarket,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<Solend<'a, 'b>, ProgramError> {
        let reserve_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let reserve_liquidity_supply_info =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let lending_market_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let lending_market_authority_info = AccountLoader::next_unchecked(account_info_iter)?;
        let reserve_liquidity_pyth_oracle_info = AccountLoader::next_unchecked(account_info_iter)?;
        let reserve_liquidity_switchboard_oracle_info =
            AccountLoader::next_unchecked(account_info_iter)?;

        assert_valid_money_market(
            money_market,
            &money_market_program_id,
            lending_market_info.key,
        )?;

        Ok(Solend {
            money_market_program_id,
            reserve: reserve_info,
            reserve_liquidity_supply: reserve_liquidity_supply_info,
            lending_market: lending_market_info,
            lending_market_authority: lending_market_authority_info,
            reserve_liquidity_pyth_oracle: reserve_liquidity_pyth_oracle_info,
            reserve_liquidity_switchboard_oracle: reserve_liquidity_switchboard_oracle_info,
        })
    }
}

impl<'a, 'b> MoneyMarket<'b> for Solend<'a, 'b> {
    fn is_collateral_return(&self) -> bool {
        true
    }

    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_liquidity: AccountInfo<'b>,
        destination_collateral: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        solend::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_pyth_oracle.clone(),
            self.reserve_liquidity_switchboard_oracle.clone(),
            clock.clone(),
        )?;

        solend::deposit(
            &self.money_market_program_id,
            source_liquidity,
            destination_collateral.clone(),
            self.reserve.clone(),
            self.reserve_liquidity_supply.clone(),
            collateral_mint,
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            authority,
            clock,
            liquidity_amount,
            signers_seeds,
        )?;

        let collateral_amount =
            Account::unpack_unchecked(&destination_collateral.data.borrow())?.amount;

        Ok(collateral_amount)
    }

    fn money_market_redeem(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_collateral: AccountInfo<'b>,
        destination_liquidity: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        solend::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_pyth_oracle.clone(),
            self.reserve_liquidity_switchboard_oracle.clone(),
            clock.clone(),
        )?;

        solend::redeem(
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
        return Err(EverlendError::MiningNotInitialized.into());
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
        return Err(EverlendError::MiningNotInitialized.into());
    }
}
