//! Utils

use crate::money_market::{CollateralPool, CollateralStorage, MoneyMarket};
use crate::money_market::{Larix, PortFinance, SPLLending, Solend};
use crate::{
    find_transit_program_address,
    state::{InternalMining, MiningType},
};
use everlend_collateral_pool::find_pool_withdraw_authority_program_address;
use everlend_income_pools::utils::IncomePoolAccounts;
use everlend_registry::state::{RegistryPrograms, RegistryRootAccounts};
use everlend_utils::{
    abs_diff, assert_account_key, cpi, find_program_address, integrations, EverlendError,
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

const RESERVE_THRESHOLD: u64 = 2;

/// Deposit
#[allow(clippy::too_many_arguments)]
pub fn deposit<'a, 'b>(
    program_id: &Pubkey,
    registry_programs: &RegistryPrograms,
    root_accounts: &RegistryRootAccounts,
    collateral_transit: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    liquidity_transit: AccountInfo<'a>,
    liquidity_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    money_market_program: AccountInfo<'a>,
    internal_mining: AccountInfo<'a>,
    money_market_account_info_iter: &'b mut Iter<AccountInfo<'a>>,
    liquidity_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<u64, ProgramError> {
    let internal_mining_type = if internal_mining.owner == program_id {
        Some(InternalMining::unpack(&internal_mining.data.borrow())?.mining_type)
    } else {
        None
    };

    let is_mining =
        internal_mining_type.is_some() && internal_mining_type != Some(MiningType::None);

    let money_market = money_market(
        registry_programs,
        money_market_program,
        money_market_account_info_iter,
        internal_mining_type,
        liquidity_mint.key,
        authority.key,
    )?;

    let collateral_amount = if is_mining {
        msg!("Deposit to Money market and deposit Mining");
        let collateral_amount = money_market.money_market_deposit_and_deposit_mining(
            collateral_mint.clone(),
            liquidity_transit.clone(),
            collateral_transit.clone(),
            authority.clone(),
            clock.clone(),
            liquidity_amount,
            signers_seeds,
        )?;

        collateral_amount
    } else {
        msg!("Deposit to Money market");
        let collateral_amount = money_market.money_market_deposit(
            collateral_mint.clone(),
            liquidity_transit.clone(),
            collateral_transit.clone(),
            authority.clone(),
            clock.clone(),
            liquidity_amount,
            signers_seeds,
        )?;

        // TODO check collateral_amount
        if collateral_amount == 0 {
            return Ok(collateral_amount);
        }

        msg!("Deposit into collateral pool");
        let coll_pool = CollateralPool::init(
            registry_programs,
            root_accounts,
            collateral_mint,
            authority.clone(),
            money_market_account_info_iter,
            false,
        )?;
        coll_pool.deposit_collateral_tokens(
            collateral_transit.clone(),
            authority,
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        collateral_amount
    };

    Ok(collateral_amount)
}

/// Withdraw
#[allow(clippy::too_many_arguments)]
pub fn withdraw<'a, 'b>(
    program_id: &Pubkey,
    registry_programs: &RegistryPrograms,
    root_accounts: &RegistryRootAccounts,
    income_pool_accounts: IncomePoolAccounts<'a>,
    collateral_transit: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    liquidity_transit: AccountInfo<'a>,
    liquidity_reserve_transit: AccountInfo<'a>,
    liquidity_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    money_market_program: AccountInfo<'a>,
    internal_mining: AccountInfo<'a>,
    money_market_account_info_iter: &'b mut Iter<AccountInfo<'a>>,
    collateral_amount: u64,
    expected_liquidity_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let liquidity_transit_supply = Account::unpack(&liquidity_transit.data.borrow())?.amount;

    let internal_mining_type = if internal_mining.owner == program_id {
        Some(InternalMining::unpack(&internal_mining.data.borrow())?.mining_type)
    } else {
        None
    };

    let is_mining =
        internal_mining_type.is_some() && internal_mining_type != Some(MiningType::None);

    let money_market = money_market(
        registry_programs,
        money_market_program,
        money_market_account_info_iter,
        internal_mining_type,
        liquidity_mint.key,
        authority.key,
    )?;

    if is_mining {
        msg!("Withdraw from Mining and Redeem from Money market");
        money_market.money_market_redeem_and_withdraw_mining(
            collateral_mint.clone(),
            collateral_transit.clone(),
            liquidity_transit.clone(),
            authority.clone(),
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )?;
    } else {
        msg!("Withdraw from collateral pool");
        let coll_pool = CollateralPool::init(
            registry_programs,
            root_accounts,
            collateral_mint.clone(),
            authority.clone(),
            money_market_account_info_iter,
            true,
        )?;

        coll_pool.withdraw_collateral_tokens(
            collateral_transit.clone(),
            authority.clone(),
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        msg!("Redeem from Money market");
        money_market.money_market_redeem(
            collateral_mint,
            collateral_transit.clone(),
            liquidity_transit.clone(),
            authority.clone(),
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )?;
    };

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
    match received_amount.cmp(&expected_liquidity_amount) {
        Ordering::Greater => {
            msg!("income_amount: {}", diff);
            everlend_income_pools::cpi::deposit(
                income_pool_accounts,
                liquidity_transit.clone(),
                authority.clone(),
                diff,
                signers_seeds,
            )?;
        }
        Ordering::Less => {
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
        }
        Ordering::Equal => {}
    }

    Ok(())
}

/// Money market
pub fn money_market<'a, 'b>(
    registry_programs: &RegistryPrograms,
    money_market_program: AccountInfo<'a>,
    money_market_account_info_iter: &'b mut Iter<AccountInfo<'a>>,
    internal_mining_type: Option<MiningType>,
    token_mint: &Pubkey,
    depositor_authority: &Pubkey,
) -> Result<Box<dyn MoneyMarket<'a> + 'a>, ProgramError> {
    let port_finance_program_id = registry_programs.money_market_program_ids[0];
    let larix_program_id = registry_programs.money_market_program_ids[1];
    let solend_program_id = registry_programs.money_market_program_ids[2];

    // Only for tests
    if money_market_program.key.to_string() == integrations::SPL_TOKEN_LENDING_PROGRAM_ID {
        let spl = SPLLending::init(*money_market_program.key, money_market_account_info_iter)?;
        return Ok(Box::new(spl));
    }

    if *money_market_program.key == port_finance_program_id {
        let port = PortFinance::init(
            *money_market_program.key,
            money_market_account_info_iter,
            internal_mining_type,
            token_mint,
            depositor_authority
        )?;
        return Ok(Box::new(port));
    }

    if *money_market_program.key == larix_program_id {
        let larix = Larix::init(
            *money_market_program.key,
            money_market_account_info_iter,
            internal_mining_type,
        )?;
        return Ok(Box::new(larix));
    }

    if *money_market_program.key == solend_program_id {
        let solend = Solend::init(*money_market_program.key, money_market_account_info_iter)?;
        return Ok(Box::new(solend));
    }

    Err(EverlendError::IncorrectInstructionProgramId.into())
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

/// ELD Fill reward accounts for token container
#[derive(Clone)]
pub struct FillRewardAccounts<'a> {
    /// Rewards mint tokne
    pub reward_mint_info: AccountInfo<'a>,
    /// Reward transit account
    pub reward_transit_info: AccountInfo<'a>,
    /// Reward vault
    pub vault_info: AccountInfo<'a>,
    /// Reward fee account
    pub fee_account_info: AccountInfo<'a>,
}

/// Collateral pool deposit account
#[allow(clippy::too_many_arguments)]
pub fn parse_fill_reward_accounts<'a>(
    program_id: &Pubkey,
    depositor_id: &Pubkey,
    reward_pool_id: &Pubkey,
    eld_reward_program_id: &Pubkey,
    account_info_iter: &mut Iter<AccountInfo<'a>>,
    check_transit_reward_destination: bool,
) -> Result<FillRewardAccounts<'a>, ProgramError> {
    let reward_mint_info = next_account_info(account_info_iter)?;
    let reward_transit_info = next_account_info(account_info_iter)?;

    // Check rewards destination account only if needed
    if check_transit_reward_destination {
        let (reward_token_account, _) = find_transit_program_address(
            program_id,
            depositor_id,
            reward_mint_info.key,
            "lm_reward",
        );
        assert_account_key(reward_transit_info, &reward_token_account)?;
    }

    let vault_info = next_account_info(account_info_iter)?;
    let fee_account_info = next_account_info(account_info_iter)?;

    let (vault, _) = Pubkey::find_program_address(
        &[
            b"vault".as_ref(),
            &reward_pool_id.to_bytes(),
            &reward_mint_info.key.to_bytes(),
        ],
        eld_reward_program_id,
    );
    assert_account_key(vault_info, &vault)?;

    Ok(FillRewardAccounts {
        reward_mint_info: reward_mint_info.clone(),
        reward_transit_info: reward_transit_info.clone(),
        vault_info: vault_info.clone(),
        fee_account_info: fee_account_info.clone(),
    })
}
