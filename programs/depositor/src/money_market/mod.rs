//! Lending money markets

use crate::utils::RESERVE_THRESHOLD;
use everlend_utils::{abs_diff, cpi, EverlendError};
use solana_program::program_pack::Pack;
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError};
use spl_token::state::Account;

mod collateral_pool;
mod francium;
mod jet;
mod larix;
mod port_finance;
mod quarry;
mod solend;
mod spl_lending;
mod tulip;

pub use collateral_pool::*;
pub use francium::*;
pub use jet::*;
pub use larix::*;
pub use port_finance::*;
pub use solend::*;
pub use spl_lending::*;
pub use tulip::*;

///
pub trait CollateralStorage<'a> {
    /// Deposit collateral tokens
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError>;
    /// Withdraw collateral tokens
    fn withdraw_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError>;
}

///
pub trait MoneyMarket<'a> {
    ///
    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'a>,
        source_liquidity: AccountInfo<'a>,
        destination_collateral: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError>;
    ///
    fn money_market_redeem(
        &self,
        collateral_mint: AccountInfo<'a>,
        source_collateral: AccountInfo<'a>,
        destination_liquidity: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError>;
    ///
    fn money_market_deposit_and_deposit_mining(
        &self,
        collateral_mint: AccountInfo<'a>,
        source_liquidity: AccountInfo<'a>,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError>;
    ///
    fn money_market_redeem_and_withdraw_mining(
        &self,
        collateral_mint: AccountInfo<'a>,
        collateral_transit: AccountInfo<'a>,
        liquidity_destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError>;
    ///
    fn refresh_income(
        &self,
        liquidity_reserve_transit: AccountInfo<'a>,
        collateral_mint: AccountInfo<'a>,
        liquidity_transit: AccountInfo<'a>,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        expected_liquidity_amount: u64,
        deposit_liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(u64, u64), ProgramError> {
        let liquidity_transit_supply = Account::unpack(&liquidity_transit.data.borrow())?.amount;

        msg!("Redeem from Money market");
        self.money_market_redeem(
            collateral_mint.clone(),
            collateral_transit.clone(),
            liquidity_transit.clone(),
            authority.clone(),
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        let received_amount = Account::unpack(&liquidity_transit.data.borrow())?
            .amount
            .checked_sub(liquidity_transit_supply)
            .ok_or(EverlendError::MathOverflow)?;
        msg!("received_amount: {}", received_amount);
        msg!("expected_liquidity_amount: {}", expected_liquidity_amount);

        // Received liquidity amount may be less
        // https://blog.neodyme.io/posts/lending_disclosure
        let diff = abs_diff(received_amount, expected_liquidity_amount)?;

        // Deposit to income pool if income amount > 0
        let income_amount = if received_amount < expected_liquidity_amount {
            msg!("income_amount: -{}", diff);
            if diff.gt(&RESERVE_THRESHOLD) {
                // throw error,  this amount is too big, probably something is wrong
                return Err(EverlendError::ReserveThreshold.into());
            }

            cpi::spl_token::transfer(
                liquidity_reserve_transit.clone(),
                liquidity_transit.clone(),
                authority.clone(),
                diff,
                signers_seeds,
            )?;
            0
        } else {
            msg!("income_amount: {}", diff);
            diff
        };

        if deposit_liquidity_amount == 0 {
            return Ok((0, income_amount));
        }

        msg!("Deposit to Money market");
        let collateral_amount = self.money_market_deposit(
            collateral_mint.clone(),
            liquidity_transit.clone(),
            collateral_transit.clone(),
            authority.clone(),
            clock.clone(),
            deposit_liquidity_amount,
            signers_seeds,
        )?;

        if collateral_amount == 0 {
            return Err(EverlendError::CollateralLeak.into());
        }

        Ok((collateral_amount, income_amount))
    }
    ///
    fn refresh_income_with_mining(
        &self,
        liquidity_reserve_transit: AccountInfo<'a>,
        collateral_mint: AccountInfo<'a>,
        liquidity_transit: AccountInfo<'a>,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        expected_liquidity_amount: u64,
        deposit_liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(u64, u64), ProgramError> {
        let liquidity_transit_supply = Account::unpack(&liquidity_transit.data.borrow())?.amount;

        msg!("Withdraw from Mining and Redeem from Money market");
        self.money_market_redeem_and_withdraw_mining(
            collateral_mint.clone(),
            collateral_transit.clone(),
            liquidity_transit.clone(),
            authority.clone(),
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        let received_amount = Account::unpack(&liquidity_transit.data.borrow())?
            .amount
            .checked_sub(liquidity_transit_supply)
            .ok_or(EverlendError::MathOverflow)?;
        msg!("received_amount: {}", received_amount);
        msg!("expected_liquidity_amount: {}", expected_liquidity_amount);

        // Received liquidity amount may be less
        // https://blog.neodyme.io/posts/lending_disclosure
        let diff = abs_diff(received_amount, expected_liquidity_amount)?;

        // Deposit to income pool if income amount > 0
        let income_amount = if received_amount < expected_liquidity_amount {
            msg!("income_amount: -{}", diff);
            if diff.gt(&RESERVE_THRESHOLD) {
                // throw error,  this amount is too big, probably something is wrong
                return Err(EverlendError::ReserveThreshold.into());
            }

            cpi::spl_token::transfer(
                liquidity_reserve_transit.clone(),
                liquidity_transit.clone(),
                authority.clone(),
                diff,
                signers_seeds,
            )?;
            0
        } else {
            msg!("income_amount: {}", diff);
            diff
        };

        if deposit_liquidity_amount == 0 {
            return Ok((0, income_amount));
        }

        msg!("Deposit to Money market and deposit Mining");
        let collateral_amount = self.money_market_deposit_and_deposit_mining(
            collateral_mint.clone(),
            liquidity_transit.clone(),
            collateral_transit.clone(),
            authority.clone(),
            clock.clone(),
            deposit_liquidity_amount,
            signers_seeds,
        )?;

        Ok((collateral_amount, income_amount))
    }
}
