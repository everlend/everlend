//! Utils

use everlend_registry::state::RegistryPrograms;
use everlend_utils::{cpi, integrations, EverlendError};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
};
use spl_token::state::Account;
use std::{cmp::Ordering, slice::Iter};

/// Deposit
#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    registry_programs: &RegistryPrograms,
    mm_pool_market: AccountInfo<'a>,
    mm_pool_market_authority: AccountInfo<'a>,
    mm_pool: AccountInfo<'a>,
    mm_pool_token_account: AccountInfo<'a>,
    mm_pool_collateral_mint: AccountInfo<'a>,
    collateral_transit: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    liquidity_transit: AccountInfo<'a>,
    liquidity_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    money_market_program: AccountInfo<'a>,
    money_market_account_info_iter: &mut Iter<AccountInfo<'a>>,
    liquidity_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<u64, ProgramError> {
    msg!("Deposit to Money market");
    money_market_deposit(
        registry_programs,
        money_market_program.clone(),
        liquidity_transit.clone(),
        liquidity_mint.clone(),
        collateral_transit.clone(),
        collateral_mint.clone(),
        authority.clone(),
        money_market_account_info_iter,
        clock.clone(),
        liquidity_amount,
        signers_seeds,
    )?;

    let collateral_amount = Account::unpack_unchecked(&collateral_transit.data.borrow())?.amount;

    msg!("Collect collateral tokens to MM Pool");
    everlend_collateral_pool::cpi::deposit(
        mm_pool_market.clone(),
        mm_pool_market_authority.clone(),
        mm_pool.clone(),
        collateral_transit.clone(),
        mm_pool_token_account.clone(),
        mm_pool_collateral_mint.clone(),
        authority.clone(),
        collateral_amount,
        signers_seeds,
    )?;

    Ok(collateral_amount)
}

/// Withdraw
#[allow(clippy::too_many_arguments)]
pub fn withdraw<'a>(
    registry_programs: &RegistryPrograms,
    income_pool_market: AccountInfo<'a>,
    income_pool: AccountInfo<'a>,
    income_pool_token_account: AccountInfo<'a>,
    mm_pool_market: AccountInfo<'a>,
    mm_pool_market_authority: AccountInfo<'a>,
    mm_pool: AccountInfo<'a>,
    mm_pool_token_account: AccountInfo<'a>,
    mm_pool_withdraw_authrity: AccountInfo<'a>,
    mm_pool_collateral_mint: AccountInfo<'a>,
    collateral_transit: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    liquidity_transit: AccountInfo<'a>,
    liquidity_reserve_transit: AccountInfo<'a>,
    liquidity_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    money_market_program: AccountInfo<'a>,
    money_market_account_info_iter: &mut Iter<AccountInfo<'a>>,
    collateral_amount: u64,
    liquidity_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let liquidity_transit_supply = Account::unpack(&liquidity_transit.data.borrow())?.amount;

    msg!("Withdraw collateral tokens from MM Pool");
    everlend_collateral_pool::cpi::withdraw(
        mm_pool_market.clone(),
        mm_pool_market_authority.clone(),
        mm_pool.clone(),
        mm_pool_withdraw_authrity.clone(),
        mm_pool_token_account.clone(),
        mm_pool_collateral_mint.clone(),
        authority.clone(),
        collateral_amount,
        signers_seeds,
    )?;

    msg!("Redeem from Money market");
    money_market_redeem(
        registry_programs,
        money_market_program.clone(),
        collateral_transit.clone(),
        collateral_mint.clone(),
        liquidity_transit.clone(),
        liquidity_mint.clone(),
        authority.clone(),
        money_market_account_info_iter,
        clock.clone(),
        collateral_amount,
        signers_seeds,
    )?;

    let received_amount = Account::unpack(&liquidity_transit.data.borrow())?
        .amount
        .checked_sub(liquidity_transit_supply)
        .ok_or(EverlendError::MathOverflow)?;
    msg!("received_amount: {}", received_amount);
    msg!("liquidity_amount: {}", liquidity_amount);

    // Received liquidity amount may be less
    // https://blog.neodyme.io/posts/lending_disclosure
    let income_amount: i64 = (received_amount as i64)
        .checked_sub(liquidity_amount as i64)
        .ok_or(EverlendError::MathOverflow)?;
    msg!("income_amount: {}", income_amount);

    // Deposit to income pool if income amount > 0
    match income_amount.cmp(&0) {
        Ordering::Greater => {
            everlend_income_pools::cpi::deposit(
                income_pool_market.clone(),
                income_pool.clone(),
                liquidity_transit.clone(),
                income_pool_token_account.clone(),
                authority.clone(),
                income_amount as u64,
                signers_seeds,
            )?;
        }
        Ordering::Less => {
            cpi::spl_token::transfer(
                liquidity_reserve_transit.clone(),
                liquidity_transit.clone(),
                authority.clone(),
                income_amount
                    .checked_abs()
                    .ok_or(EverlendError::MathOverflow)? as u64,
                signers_seeds,
            )?;
        }
        Ordering::Equal => {}
    }

    Ok(())
}

/// Money market deposit
#[allow(clippy::too_many_arguments)]
pub fn money_market_deposit<'a>(
    registry_programs: &RegistryPrograms,
    money_market_program: AccountInfo<'a>,
    source_liquidity: AccountInfo<'a>,
    _liquidity_mint: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    money_market_account_info_iter: &mut Iter<AccountInfo<'a>>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let port_finance_program_id = registry_programs.money_market_program_ids[0];
    let larix_program_id = registry_programs.money_market_program_ids[1];
    let solend_program_id = registry_programs.money_market_program_ids[2];

    // Only for tests
    if money_market_program.key.to_string() == integrations::SPL_TOKEN_LENDING_PROGRAM_ID {
        let reserve_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_oracle_info = next_account_info(money_market_account_info_iter)?;

        cpi::spl_token_lending::refresh_reserve(
            money_market_program.key,
            reserve_info.clone(),
            reserve_liquidity_oracle_info.clone(),
            clock.clone(),
        )?;

        return cpi::spl_token_lending::deposit(
            money_market_program.key,
            source_liquidity.clone(),
            destination_collateral.clone(),
            reserve_info.clone(),
            reserve_liquidity_supply_info.clone(),
            collateral_mint.clone(),
            lending_market_info.clone(),
            lending_market_authority_info.clone(),
            authority.clone(),
            clock.clone(),
            amount,
            signers_seeds,
        );
    }

    if *money_market_program.key == port_finance_program_id {
        let reserve_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_oracle_info = next_account_info(money_market_account_info_iter)?;

        cpi::port_finance::refresh_reserve(
            money_market_program.key,
            reserve_info.clone(),
            reserve_liquidity_oracle_info.clone(),
            clock.clone(),
        )?;

        cpi::port_finance::deposit(
            money_market_program.key,
            source_liquidity.clone(),
            destination_collateral.clone(),
            reserve_info.clone(),
            reserve_liquidity_supply_info.clone(),
            collateral_mint.clone(),
            lending_market_info.clone(),
            lending_market_authority_info.clone(),
            authority.clone(),
            clock.clone(),
            amount,
            signers_seeds,
        )
    } else if *money_market_program.key == larix_program_id {
        let reserve_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_oracle_info = next_account_info(money_market_account_info_iter)?;

        cpi::larix::refresh_reserve(
            money_market_program.key,
            reserve_info.clone(),
            reserve_liquidity_oracle_info.clone(),
        )?;

        cpi::larix::deposit(
            money_market_program.key,
            source_liquidity.clone(),
            destination_collateral.clone(),
            reserve_info.clone(),
            reserve_liquidity_supply_info.clone(),
            collateral_mint.clone(),
            lending_market_info.clone(),
            lending_market_authority_info.clone(),
            authority.clone(),
            amount,
            signers_seeds,
        )
    } else if *money_market_program.key == solend_program_id {
        let reserve_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_pyth_oracle_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_switchboard_oracle_info =
            next_account_info(money_market_account_info_iter)?;

        cpi::solend::refresh_reserve(
            money_market_program.key,
            reserve_info.clone(),
            reserve_liquidity_pyth_oracle_info.clone(),
            reserve_liquidity_switchboard_oracle_info.clone(),
            clock.clone(),
        )?;

        cpi::solend::deposit(
            money_market_program.key,
            source_liquidity.clone(),
            destination_collateral.clone(),
            reserve_info.clone(),
            reserve_liquidity_supply_info.clone(),
            collateral_mint.clone(),
            lending_market_info.clone(),
            lending_market_authority_info.clone(),
            authority.clone(),
            clock.clone(),
            amount,
            signers_seeds,
        )
    } else {
        Err(EverlendError::IncorrectInstructionProgramId.into())
    }
}

/// Money market redeem
#[allow(clippy::too_many_arguments)]
pub fn money_market_redeem<'a>(
    registry_programs: &RegistryPrograms,
    money_market_program: AccountInfo<'a>,
    source_collateral: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    destination_liquidity: AccountInfo<'a>,
    _liquidity_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    money_market_account_info_iter: &mut Iter<AccountInfo<'a>>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let port_finance_program_id = registry_programs.money_market_program_ids[0];
    let larix_program_id = registry_programs.money_market_program_ids[1];
    let solend_program_id = registry_programs.money_market_program_ids[2];

    // Only for tests
    if money_market_program.key.to_string() == integrations::SPL_TOKEN_LENDING_PROGRAM_ID {
        let reserve_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_oracle_info = next_account_info(money_market_account_info_iter)?;

        cpi::spl_token_lending::refresh_reserve(
            money_market_program.key,
            reserve_info.clone(),
            reserve_liquidity_oracle_info.clone(),
            clock.clone(),
        )?;

        return cpi::spl_token_lending::redeem(
            money_market_program.key,
            source_collateral.clone(),
            destination_liquidity.clone(),
            reserve_info.clone(),
            collateral_mint.clone(),
            reserve_liquidity_supply_info.clone(),
            lending_market_info.clone(),
            lending_market_authority_info.clone(),
            authority.clone(),
            clock.clone(),
            amount,
            signers_seeds,
        );
    }

    if *money_market_program.key == port_finance_program_id {
        let reserve_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_oracle_info = next_account_info(money_market_account_info_iter)?;

        cpi::port_finance::refresh_reserve(
            money_market_program.key,
            reserve_info.clone(),
            reserve_liquidity_oracle_info.clone(),
            clock.clone(),
        )?;

        cpi::port_finance::redeem(
            money_market_program.key,
            source_collateral.clone(),
            destination_liquidity.clone(),
            reserve_info.clone(),
            collateral_mint.clone(),
            reserve_liquidity_supply_info.clone(),
            lending_market_info.clone(),
            lending_market_authority_info.clone(),
            authority.clone(),
            clock.clone(),
            amount,
            signers_seeds,
        )
    } else if *money_market_program.key == larix_program_id {
        let reserve_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_oracle_info = next_account_info(money_market_account_info_iter)?;

        cpi::larix::refresh_reserve(
            money_market_program.key,
            reserve_info.clone(),
            reserve_liquidity_oracle_info.clone(),
        )?;

        cpi::larix::redeem(
            money_market_program.key,
            source_collateral.clone(),
            destination_liquidity.clone(),
            reserve_info.clone(),
            collateral_mint.clone(),
            reserve_liquidity_supply_info.clone(),
            lending_market_info.clone(),
            lending_market_authority_info.clone(),
            authority.clone(),
            amount,
            signers_seeds,
        )
    } else if *money_market_program.key == solend_program_id {
        let reserve_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_pyth_oracle_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_switchboard_oracle_info =
            next_account_info(money_market_account_info_iter)?;

        cpi::solend::refresh_reserve(
            money_market_program.key,
            reserve_info.clone(),
            reserve_liquidity_pyth_oracle_info.clone(),
            reserve_liquidity_switchboard_oracle_info.clone(),
            clock.clone(),
        )?;

        cpi::solend::redeem(
            money_market_program.key,
            source_collateral.clone(),
            destination_liquidity.clone(),
            reserve_info.clone(),
            collateral_mint.clone(),
            reserve_liquidity_supply_info.clone(),
            lending_market_info.clone(),
            lending_market_authority_info.clone(),
            authority.clone(),
            clock.clone(),
            amount,
            signers_seeds,
        )
    } else {
        Err(EverlendError::IncorrectInstructionProgramId.into())
    }
}
