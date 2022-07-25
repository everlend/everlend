//! Utils

use crate::state::{InternalMining, MiningType};
use everlend_collateral_pool::{
    find_pool_withdraw_authority_program_address, utils::CollateralPoolAccounts,
};
use everlend_income_pools::utils::IncomePoolAccounts;
use everlend_registry::state::{RegistryPrograms, RegistryRootAccounts};
use everlend_utils::{
    assert_account_key, assert_owned_by, cpi, find_program_address, integrations, EverlendError,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::AccountMeta,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{cmp::Ordering, slice::Iter};

/// Deposit
#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    program_id: &Pubkey,
    registry_programs: &RegistryPrograms,
    root_accounts: &RegistryRootAccounts,
    collateral_transit: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    liquidity_transit: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    money_market_program: AccountInfo<'a>,
    internal_mining: AccountInfo<'a>,
    money_market_account_info_iter: &mut Iter<AccountInfo<'a>>,
    liquidity_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<u64, ProgramError> {
    let reserve_info = next_account_info(money_market_account_info_iter)?;
    let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
    let lending_market_info = next_account_info(money_market_account_info_iter)?;
    let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;
    let reserve_liquidity_oracle_info = next_account_info(money_market_account_info_iter)?;
    let internal_mining_type = if internal_mining.owner == program_id {
        Some(InternalMining::unpack(&internal_mining.data.borrow())?.mining_type)
    } else {
        None
    };

    msg!("Deposit to Money market");
    money_market_deposit(
        registry_programs,
        money_market_program.clone(),
        liquidity_transit.clone(),
        collateral_transit.clone(),
        collateral_mint.clone(),
        authority.clone(),
        money_market_account_info_iter,
        reserve_info.clone(),
        reserve_liquidity_supply_info.clone(),
        lending_market_info.clone(),
        lending_market_authority_info.clone(),
        reserve_liquidity_oracle_info.clone(),
        clock.clone(),
        liquidity_amount,
        signers_seeds,
    )?;

    let collateral_amount = Account::unpack_unchecked(&collateral_transit.data.borrow())?.amount;

    match internal_mining_type {
        Some(MiningType::Larix { mining_account, .. }) => {
            let reserve_bonus_info = next_account_info(money_market_account_info_iter)?;
            let mining_info = next_account_info(money_market_account_info_iter)?;
            assert_account_key(mining_info, &mining_account)?;

            cpi::larix::refresh_reserve(
                money_market_program.key,
                reserve_info.clone(),
                reserve_liquidity_oracle_info.clone(),
            )?;

            cpi::larix::deposit_mining(
                money_market_program.key,
                collateral_transit.clone(),
                reserve_bonus_info.clone(),
                mining_info.clone(),
                reserve_info.clone(),
                lending_market_info.clone(),
                authority.clone(),
                authority,
                collateral_amount,
                signers_seeds,
            )?;
        }
        Some(MiningType::Quarry {
            quarry_mining_program_id,
            quarry,
            rewarder,
            token_mint: _,
            miner_vault,
        }) => {
            let quarry_mining_program_id_info = next_account_info(money_market_account_info_iter)?;
            let miner_info = next_account_info(money_market_account_info_iter)?;
            let quarry_info = next_account_info(money_market_account_info_iter)?;
            let rewarder_info = next_account_info(money_market_account_info_iter)?;
            let miner_vault_info = next_account_info(money_market_account_info_iter)?;
            assert_account_key(quarry_mining_program_id_info, &quarry_mining_program_id)?;
            assert_account_key(quarry_info, &quarry)?;
            assert_account_key(rewarder_info, &rewarder)?;
            assert_account_key(miner_vault_info, &miner_vault)?;
            let (miner_pubkey, _) = cpi::quarry::find_miner_program_address(
                &quarry_mining_program_id,
                &quarry,
                &internal_mining.key,
            );
            assert_account_key(miner_info, &miner_pubkey)?;
            cpi::quarry::stake_tokens(
                &quarry_mining_program_id,
                authority.clone(),
                miner_info.clone(),
                quarry_info.clone(),
                miner_vault_info.clone(),
                collateral_transit.clone(),
                rewarder_info.clone(),
                collateral_amount,
                signers_seeds,
            )?;
        }
        Some(MiningType::PortFinance {
            staking_program_id,
            staking_account,
            staking_pool,
            obligation,
        }) => {
            let staking_program_id_info = next_account_info(money_market_account_info_iter)?;
            let staking_account_info = next_account_info(money_market_account_info_iter)?;
            let staking_pool_info = next_account_info(money_market_account_info_iter)?;

            assert_account_key(staking_program_id_info, &staking_program_id)?;
            assert_account_key(staking_account_info, &staking_account)?;
            assert_account_key(staking_pool_info, &staking_pool)?;

            let obligation_info = next_account_info(money_market_account_info_iter)?;
            assert_account_key(obligation_info, &obligation)?;

            let collateral_supply_pubkey_info = next_account_info(money_market_account_info_iter)?;

            cpi::port_finance::refresh_reserve(
                money_market_program.key,
                reserve_info.clone(),
                reserve_liquidity_oracle_info.clone(),
                clock.clone(),
            )?;

            // TODO use DepositReserveLiquidityAndObligationCollateral after refactor
            // Mining by obligation
            cpi::port_finance::deposit_obligation_collateral(
                money_market_program.key,
                collateral_transit.clone(),
                collateral_supply_pubkey_info.clone(),
                reserve_info.clone(),
                obligation_info.clone(),
                lending_market_info.clone(),
                authority.clone(),
                authority.clone(),
                staking_account_info.clone(),
                staking_pool_info.clone(),
                staking_program_id_info.clone(),
                lending_market_authority_info.clone(),
                clock.clone(),
                collateral_amount,
                signers_seeds,
            )?;
        }
        None | Some(MiningType::None) => {
            let collateral_pool_market_info = next_account_info(money_market_account_info_iter)?;
            let collateral_pool_market_authority_info =
                next_account_info(money_market_account_info_iter)?;
            let collateral_pool_info = next_account_info(money_market_account_info_iter)?;
            let collateral_pool_token_account_info =
                next_account_info(money_market_account_info_iter)?;

            // Check external programs
            assert_owned_by(
                collateral_pool_market_info,
                &registry_programs.collateral_pool_program_id,
            )?;
            assert_owned_by(
                collateral_pool_info,
                &registry_programs.collateral_pool_program_id,
            )?;

            // Check collateral pool market
            if !root_accounts
                .collateral_pool_markets
                .contains(collateral_pool_market_info.key)
            {
                return Err(ProgramError::InvalidArgument);
            }

            // Check collateral pool
            let (collateral_pool_pubkey, _) = everlend_collateral_pool::find_pool_program_address(
                &registry_programs.collateral_pool_program_id,
                collateral_pool_market_info.key,
                collateral_mint.key,
            );
            assert_account_key(collateral_pool_info, &collateral_pool_pubkey)?;

            let collateral_pool =
                everlend_collateral_pool::state::Pool::unpack(&collateral_pool_info.data.borrow())?;

            // Check collateral pool accounts
            assert_account_key(&collateral_mint, &collateral_pool.token_mint)?;
            assert_account_key(
                collateral_pool_token_account_info,
                &collateral_pool.token_account,
            )?;

            let _everlend_collateral_pool_info = next_account_info(money_market_account_info_iter)?;

            let collateral_pool_accounts = CollateralPoolAccounts {
                pool_market: collateral_pool_market_info.clone(),
                pool_market_authority: collateral_pool_market_authority_info.clone(),
                pool: collateral_pool_info.clone(),
                token_account: collateral_pool_token_account_info.clone(),
            };
            msg!("Collect collateral tokens to MM Pool");
            everlend_collateral_pool::cpi::deposit(
                collateral_pool_accounts,
                collateral_transit.clone(),
                authority.clone(),
                collateral_amount,
                signers_seeds,
            )?;
        }
    }
    Ok(collateral_amount)
}

/// Withdraw
#[allow(clippy::too_many_arguments)]
pub fn withdraw<'a>(
    program_id: &Pubkey,
    registry_programs: &RegistryPrograms,
    root_accounts: &RegistryRootAccounts,
    income_pool_accounts: IncomePoolAccounts<'a>,
    collateral_transit: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    liquidity_transit: AccountInfo<'a>,
    liquidity_reserve_transit: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    money_market_program: AccountInfo<'a>,
    internal_mining: AccountInfo<'a>,
    money_market_account_info_iter: &mut Iter<AccountInfo<'a>>,
    collateral_amount: u64,
    liquidity_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let reserve_info = next_account_info(money_market_account_info_iter)?;
    let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
    let lending_market_info = next_account_info(money_market_account_info_iter)?;
    let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;
    let reserve_liquidity_oracle_info = next_account_info(money_market_account_info_iter)?;

    let liquidity_transit_supply = Account::unpack(&liquidity_transit.data.borrow())?.amount;

    let internal_mining_type = if internal_mining.owner == program_id {
        Some(InternalMining::unpack(&internal_mining.data.borrow())?.mining_type)
    } else {
        None
    };

    match internal_mining_type {
        Some(MiningType::Larix { mining_account, .. }) => {
            let reserve_bonus_info = next_account_info(money_market_account_info_iter)?;
            let mining_info = next_account_info(money_market_account_info_iter)?;
            assert_account_key(mining_info, &mining_account)?;

            cpi::larix::refresh_reserve(
                money_market_program.key,
                reserve_info.clone(),
                reserve_liquidity_oracle_info.clone(),
            )?;

            cpi::larix::withdraw_mining(
                money_market_program.key,
                collateral_transit.clone(),
                reserve_bonus_info.clone(),
                mining_info.clone(),
                reserve_info.clone(),
                lending_market_info.clone(),
                lending_market_authority_info.clone(),
                authority.clone(),
                clock.clone(),
                collateral_amount,
                signers_seeds,
            )?;
        }
        Some(MiningType::Quarry {
            quarry_mining_program_id,
            quarry,
            rewarder,
            token_mint: _,
            miner_vault,
        }) => {
            let miner_info = next_account_info(money_market_account_info_iter)?;
            let quarry_info = next_account_info(money_market_account_info_iter)?;
            let rewarder_info = next_account_info(money_market_account_info_iter)?;
            let miner_vault_info = next_account_info(money_market_account_info_iter)?;
            assert_account_key(quarry_info, &quarry)?;
            assert_account_key(rewarder_info, &rewarder)?;
            assert_account_key(miner_vault_info, &miner_vault)?;
            let (miner_pubkey, _miner_bump) = cpi::quarry::find_miner_program_address(
                &quarry_mining_program_id,
                &quarry,
                internal_mining.key,
            );
            assert_account_key(miner_info, &miner_pubkey)?;
            cpi::quarry::withdraw_tokens(
                &quarry_mining_program_id,
                authority.clone(),
                miner_info.clone(),
                quarry_info.clone(),
                miner_vault_info.clone(),
                collateral_transit.clone(),
                rewarder_info.clone(),
                collateral_amount,
                signers_seeds,
            )?;
        }
        Some(MiningType::PortFinance {
            staking_program_id,
            staking_account,
            staking_pool,
            obligation,
        }) => {
            let staking_program_id_info = next_account_info(money_market_account_info_iter)?;
            let staking_account_info = next_account_info(money_market_account_info_iter)?;
            let staking_pool_info = next_account_info(money_market_account_info_iter)?;

            assert_account_key(staking_program_id_info, &staking_program_id)?;
            assert_account_key(staking_account_info, &staking_account)?;
            assert_account_key(staking_pool_info, &staking_pool)?;

            let obligation_info = next_account_info(money_market_account_info_iter)?;
            assert_account_key(obligation_info, &obligation)?;

            let collateral_supply_pubkey_info = next_account_info(money_market_account_info_iter)?;

            cpi::port_finance::refresh_reserve(
                money_market_program.key,
                reserve_info.clone(),
                reserve_liquidity_oracle_info.clone(),
                clock.clone(),
            )?;

            cpi::port_finance::refresh_obligation(
                money_market_program.key,
                obligation_info.clone(),
                reserve_info.clone(),
                clock.clone(),
            )?;

            // Mining by obligation
            cpi::port_finance::withdraw_obligation_collateral(
                money_market_program.key,
                collateral_supply_pubkey_info.clone(),
                collateral_transit.clone(),
                reserve_info.clone(),
                obligation_info.clone(),
                lending_market_info.clone(),
                authority.clone(),
                staking_account_info.clone(),
                staking_pool_info.clone(),
                staking_program_id_info.clone(),
                lending_market_authority_info.clone(),
                clock.clone(),
                collateral_amount,
                signers_seeds,
            )?;
        }
        None | Some(MiningType::None) => {
            let collateral_pool_market_info = next_account_info(money_market_account_info_iter)?;
            let collateral_pool_market_authority_info =
                next_account_info(money_market_account_info_iter)?;
            let collateral_pool_info = next_account_info(money_market_account_info_iter)?;
            let collateral_pool_token_account_info =
                next_account_info(money_market_account_info_iter)?;

            // Check external programs
            assert_owned_by(
                collateral_pool_market_info,
                &registry_programs.collateral_pool_program_id,
            )?;
            assert_owned_by(
                collateral_pool_info,
                &registry_programs.collateral_pool_program_id,
            )?;

            // Check collateral pool market
            if !root_accounts
                .collateral_pool_markets
                .contains(collateral_pool_market_info.key)
            {
                return Err(ProgramError::InvalidArgument);
            }

            // Check collateral pool
            let (collateral_pool_pubkey, _) = everlend_collateral_pool::find_pool_program_address(
                &registry_programs.collateral_pool_program_id,
                collateral_pool_market_info.key,
                collateral_mint.key,
            );
            assert_account_key(collateral_pool_info, &collateral_pool_pubkey)?;

            let collateral_pool =
                everlend_collateral_pool::state::Pool::unpack(&collateral_pool_info.data.borrow())?;
            //
            // Check collateral pool accounts
            assert_account_key(&collateral_mint, &collateral_pool.token_mint)?;
            assert_account_key(
                collateral_pool_token_account_info,
                &collateral_pool.token_account,
            )?;

            let collateral_pool_withdraw_authority_info =
                next_account_info(money_market_account_info_iter)?;

            let (collateral_pool_withdraw_authority, _) =
                find_pool_withdraw_authority_program_address(
                    &registry_programs.collateral_pool_program_id,
                    collateral_pool_info.key,
                    authority.key,
                );
            assert_account_key(
                collateral_pool_withdraw_authority_info,
                &collateral_pool_withdraw_authority,
            )?;

            let collateral_pool_accounts = CollateralPoolAccounts {
                pool_market: collateral_pool_market_info.clone(),
                pool_market_authority: collateral_pool_market_authority_info.clone(),
                pool: collateral_pool_info.clone(),
                token_account: collateral_pool_token_account_info.clone(),
            };
            let _everlend_collateral_pool_info = next_account_info(money_market_account_info_iter)?;

            msg!("Withdraw collateral tokens from MM Pool");
            everlend_collateral_pool::cpi::withdraw(
                collateral_pool_accounts,
                collateral_pool_withdraw_authority_info.clone(),
                collateral_transit.clone(),
                authority.clone(),
                collateral_amount,
                signers_seeds,
            )?;
        }
    }

    msg!("Redeem from Money market");
    money_market_redeem(
        registry_programs,
        money_market_program.clone(),
        collateral_transit.clone(),
        collateral_mint.clone(),
        liquidity_transit.clone(),
        authority.clone(),
        money_market_account_info_iter,
        reserve_info.clone(),
        reserve_liquidity_supply_info.clone(),
        lending_market_info.clone(),
        lending_market_authority_info.clone(),
        reserve_liquidity_oracle_info.clone(),
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
                income_pool_accounts,
                liquidity_transit.clone(),
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
    destination_collateral: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    money_market_account_info_iter: &mut Iter<AccountInfo<'a>>,
    reserve_info: AccountInfo<'a>,
    reserve_liquidity_supply_info: AccountInfo<'a>,
    lending_market_info: AccountInfo<'a>,
    lending_market_authority_info: AccountInfo<'a>,
    reserve_liquidity_oracle_info: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let port_finance_program_id = registry_programs.money_market_program_ids[0];
    let larix_program_id = registry_programs.money_market_program_ids[1];
    let solend_program_id = registry_programs.money_market_program_ids[2];

    // Only for tests
    if money_market_program.key.to_string() == integrations::SPL_TOKEN_LENDING_PROGRAM_ID {
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
        let reserve_liquidity_pyth_oracle_info = reserve_liquidity_oracle_info;
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
    authority: AccountInfo<'a>,
    money_market_account_info_iter: &mut Iter<AccountInfo<'a>>,
    reserve: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    reserve_liquidity_oracle_info: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let port_finance_program_id = registry_programs.money_market_program_ids[0];
    let larix_program_id = registry_programs.money_market_program_ids[1];
    let solend_program_id = registry_programs.money_market_program_ids[2];

    // Only for tests
    if money_market_program.key.to_string() == integrations::SPL_TOKEN_LENDING_PROGRAM_ID {
        cpi::spl_token_lending::refresh_reserve(
            money_market_program.key,
            reserve.clone(),
            reserve_liquidity_oracle_info.clone(),
            clock.clone(),
        )?;

        return cpi::spl_token_lending::redeem(
            money_market_program.key,
            source_collateral.clone(),
            destination_liquidity.clone(),
            reserve.clone(),
            collateral_mint.clone(),
            reserve_liquidity_supply.clone(),
            lending_market.clone(),
            lending_market_authority.clone(),
            authority.clone(),
            clock.clone(),
            amount,
            signers_seeds,
        );
    }

    if *money_market_program.key == port_finance_program_id {
        cpi::port_finance::refresh_reserve(
            money_market_program.key,
            reserve.clone(),
            reserve_liquidity_oracle_info.clone(),
            clock.clone(),
        )?;

        cpi::port_finance::redeem(
            money_market_program.key,
            source_collateral.clone(),
            destination_liquidity.clone(),
            reserve.clone(),
            collateral_mint.clone(),
            reserve_liquidity_supply.clone(),
            lending_market.clone(),
            lending_market_authority.clone(),
            authority.clone(),
            clock.clone(),
            amount,
            signers_seeds,
        )
    } else if *money_market_program.key == larix_program_id {
        cpi::larix::refresh_reserve(
            money_market_program.key,
            reserve.clone(),
            reserve_liquidity_oracle_info.clone(),
        )?;

        cpi::larix::redeem(
            money_market_program.key,
            source_collateral.clone(),
            destination_liquidity.clone(),
            reserve.clone(),
            collateral_mint.clone(),
            reserve_liquidity_supply.clone(),
            lending_market.clone(),
            lending_market_authority.clone(),
            authority.clone(),
            amount,
            signers_seeds,
        )
    } else if *money_market_program.key == solend_program_id {
        let reserve_liquidity_pyth_oracle_info = reserve_liquidity_oracle_info;
        let reserve_liquidity_switchboard_oracle_info =
            next_account_info(money_market_account_info_iter)?;

        cpi::solend::refresh_reserve(
            money_market_program.key,
            reserve.clone(),
            reserve_liquidity_pyth_oracle_info.clone(),
            reserve_liquidity_switchboard_oracle_info.clone(),
            clock.clone(),
        )?;

        cpi::solend::redeem(
            money_market_program.key,
            source_collateral.clone(),
            destination_liquidity.clone(),
            reserve.clone(),
            collateral_mint.clone(),
            reserve_liquidity_supply.clone(),
            lending_market.clone(),
            lending_market_authority.clone(),
            authority.clone(),
            clock.clone(),
            amount,
            signers_seeds,
        )
    } else {
        Err(EverlendError::IncorrectInstructionProgramId.into())
    }
}

/// Collateral pool deposit account
#[allow(clippy::too_many_arguments)]
pub fn collateral_pool_deposit_accounts(
    pool_market: &Pubkey,
    collateral_mint: &Pubkey,
    collateral_pool_token_account: &Pubkey,
) -> Vec<AccountMeta> {
    let (collateral_pool_market_authority, _) =
        find_program_address(&everlend_collateral_pool::id(), pool_market);
    let (collateral_pool, _) = everlend_collateral_pool::find_pool_program_address(
        &everlend_collateral_pool::id(),
        pool_market,
        collateral_mint,
    );

    vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(collateral_pool_market_authority, false),
        AccountMeta::new_readonly(collateral_pool, false),
        AccountMeta::new(*collateral_pool_token_account, false),
        AccountMeta::new_readonly(everlend_collateral_pool::id(), false),
    ]
}

/// Collateral pool deposit account
#[allow(clippy::too_many_arguments)]
pub fn collateral_pool_withdraw_accounts(
    pool_market: &Pubkey,
    collateral_mint: &Pubkey,
    collateral_pool_token_account: &Pubkey,
    depositor_program_id: &Pubkey,
    depositor: &Pubkey,
) -> Vec<AccountMeta> {
    let (collateral_pool_market_authority, _) =
        find_program_address(&everlend_collateral_pool::id(), pool_market);
    let (collateral_pool, _) = everlend_collateral_pool::find_pool_program_address(
        &everlend_collateral_pool::id(),
        pool_market,
        collateral_mint,
    );

    let (depositor_authority, _) = find_program_address(depositor_program_id, depositor);

    let (collateral_pool_withdraw_authority, _) = find_pool_withdraw_authority_program_address(
        &everlend_collateral_pool::id(),
        &collateral_pool,
        &depositor_authority,
    );

    vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(collateral_pool_market_authority, false),
        AccountMeta::new_readonly(collateral_pool, false),
        AccountMeta::new(*collateral_pool_token_account, false),
        AccountMeta::new_readonly(collateral_pool_withdraw_authority, false),
        AccountMeta::new_readonly(everlend_collateral_pool::id(), false),
    ]
}
