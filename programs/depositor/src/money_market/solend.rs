use super::MoneyMarket;
use everlend_utils::{cpi::solend, EverlendError};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Account;
use std::slice::Iter;

///
pub struct Solend<'a> {
    money_market_program_id: Pubkey,
    reserve: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    reserve_liquidity_pyth_oracle: AccountInfo<'a>,
    reserve_liquidity_switchboard_oracle: AccountInfo<'a>,
}

impl<'a, 'b> Solend<'a> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        account_info_iter: &'b mut Iter<AccountInfo<'a>>,
    ) -> Result<Solend<'a>, ProgramError> {
        let reserve_info = next_account_info(account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(account_info_iter)?;
        let lending_market_info = next_account_info(account_info_iter)?;
        let lending_market_authority_info = next_account_info(account_info_iter)?;
        let reserve_liquidity_pyth_oracle_info = next_account_info(account_info_iter)?;
        let reserve_liquidity_switchboard_oracle_info = next_account_info(account_info_iter)?;

        Ok(Solend {
            money_market_program_id,
            reserve: reserve_info.clone(),
            reserve_liquidity_supply: reserve_liquidity_supply_info.clone(),
            lending_market: lending_market_info.clone(),
            lending_market_authority: lending_market_authority_info.clone(),
            reserve_liquidity_pyth_oracle: reserve_liquidity_pyth_oracle_info.clone(),
            reserve_liquidity_switchboard_oracle: reserve_liquidity_switchboard_oracle_info.clone(),
        })
    }
}

impl<'a> MoneyMarket<'a> for Solend<'a> {
    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'a>,
        source_liquidity: AccountInfo<'a>,
        destination_collateral: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
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
        collateral_mint: AccountInfo<'a>,
        source_collateral: AccountInfo<'a>,
        destination_liquidity: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
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
        _collateral_mint: AccountInfo<'a>,
        _source_liquidity: AccountInfo<'a>,
        _collateral_transit: AccountInfo<'a>,
        _authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
        _liquidity_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        return Err(EverlendError::MiningNotInitialized.into());
    }

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
    ) -> Result<(), ProgramError> {
        return Err(EverlendError::MiningNotInitialized.into());
    }
}
