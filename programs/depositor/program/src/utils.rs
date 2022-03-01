//! Utils

use everlend_registry::state::RegistryConfig;
use everlend_utils::{cpi, integrations, EverlendError};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
};
use std::slice::Iter;

/// Money market deposit
#[allow(clippy::too_many_arguments)]
pub fn money_market_deposit<'a>(
    registry_config: &RegistryConfig,
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
    let port_finance_program_id = registry_config.money_market_program_ids[0];
    let larix_program_id = registry_config.money_market_program_ids[1];

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
    } else {
        Err(EverlendError::IncorrectInstructionProgramId.into())
    }
}

/// Money market redeem
#[allow(clippy::too_many_arguments)]
pub fn money_market_redeem<'a>(
    registry_config: &RegistryConfig,
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
    let port_finance_program_id = registry_config.money_market_program_ids[0];
    let larix_program_id = registry_config.money_market_program_ids[1];

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
    } else {
        Err(EverlendError::IncorrectInstructionProgramId.into())
    }
}
