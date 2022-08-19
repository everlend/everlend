use crate::money_market::MoneyMarket;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use everlend_utils::cpi::tulip;
use everlend_utils::EverlendError;
use solana_program::program_pack::Pack;
use spl_token::state::Account;
use std::slice::Iter;

///
pub struct Tulip<'a> {
    money_market_program_id: Pubkey,
    reserve: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    reserve_liquidity_oracle: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
}

impl<'a, 'b> Tulip<'a> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        account_info_iter: &'b mut Iter<AccountInfo<'a>>,
    ) -> Result<Tulip<'a>, ProgramError> {
        let reserve_info = next_account_info(account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(account_info_iter)?;
        let lending_market_info = next_account_info(account_info_iter)?;
        let lending_market_authority_info = next_account_info(account_info_iter)?;
        let reserve_liquidity_oracle_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        Ok(Tulip {
            money_market_program_id,
            reserve: reserve_info.clone(),
            reserve_liquidity_supply: reserve_liquidity_supply_info.clone(),
            lending_market: lending_market_info.clone(),
            lending_market_authority: lending_market_authority_info.clone(),
            reserve_liquidity_oracle: reserve_liquidity_oracle_info.clone(),
            token_program: token_program_info.clone(),
        })
    }
}

impl<'a> MoneyMarket<'a> for Tulip<'a> {
    ///
    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'a>,
        source_liquidity: AccountInfo<'a>,
        destination_collateral: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        tulip::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
            clock.clone(),
        )?;

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
            self.token_program.clone(),
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
        collateral_mint: AccountInfo<'a>,
        source_collateral: AccountInfo<'a>,
        destination_liquidity: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        tulip::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
            clock.clone(),
        )?;

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
            self.token_program.clone(),
            amount,
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
        _amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        Err(EverlendError::MiningNotImplemented.into())
    }

    ///
    fn money_market_redeem_and_withdraw_mining(
        &self,
        _collateral_mint: AccountInfo<'a>,
        _collateral_transit: AccountInfo<'a>,
        _liquidity_destination: AccountInfo<'a>,
        _authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
        _amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        Err(EverlendError::MiningNotImplemented.into())
    }
}
