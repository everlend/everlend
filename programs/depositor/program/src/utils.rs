//! Utils

use std::slice::Iter;

use everlend_utils::EverlendError;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program::invoke_signed,
    program_error::ProgramError,
};

/// ULP borrow tokens
#[allow(clippy::too_many_arguments)]
pub fn ulp_borrow<'a>(
    pool_market: AccountInfo<'a>,
    pool_market_authority: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    pool_borrow_authority: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    borrow_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = everlend_ulp::instruction::borrow(
        &everlend_ulp::id(),
        pool_market.key,
        pool.key,
        pool_borrow_authority.key,
        destination.key,
        token_account.key,
        borrow_authority.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[
            pool_market,
            pool,
            pool_borrow_authority,
            pool_market_authority,
            destination,
            token_account,
            borrow_authority,
        ],
        signers_seeds,
    )
}

/// ULP deposit tokens
#[allow(clippy::too_many_arguments)]
pub fn ulp_deposit<'a>(
    pool_market: AccountInfo<'a>,
    pool_market_authority: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    source: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    pool_mint: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = everlend_ulp::instruction::deposit(
        &everlend_ulp::id(),
        pool_market.key,
        pool.key,
        source.key,
        destination.key,
        token_account.key,
        pool_mint.key,
        user_transfer_authority.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[
            pool_market,
            pool,
            source,
            destination,
            token_account,
            pool_mint,
            pool_market_authority,
            user_transfer_authority,
        ],
        signers_seeds,
    )
}

/// Money market deposit
#[allow(clippy::too_many_arguments)]
pub fn money_market_deposit<'a>(
    money_market_program: AccountInfo<'a>,
    source_liquidity: AccountInfo<'a>,
    liquidity_mint: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    collateral_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    money_market_account_info_iter: &mut Iter<AccountInfo<'a>>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    // TODO: Get money market ids from depositor account + replace to match.
    if *money_market_program.key == spl_token_lending::id() {
        let reserve_info = next_account_info(money_market_account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_info = next_account_info(money_market_account_info_iter)?;
        let lending_market_authority_info = next_account_info(money_market_account_info_iter)?;

        spl_token_lending_deposit(
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

/// SPL Token lending deposit
#[allow(clippy::too_many_arguments)]
pub fn spl_token_lending_deposit<'a>(
    source_liquidity: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    reserve_collateral_mint: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = spl_token_lending::instruction::deposit_reserve_liquidity(
        spl_token_lending::id(),
        amount,
        *source_liquidity.key,
        *destination_collateral.key,
        *reserve.key,
        *reserve_liquidity_supply.key,
        *reserve_collateral_mint.key,
        *lending_market.key,
        *authority.key,
    );

    invoke_signed(
        &ix,
        &[
            source_liquidity,
            destination_collateral,
            reserve,
            reserve_liquidity_supply,
            reserve_collateral_mint,
            lending_market,
            lending_market_authority,
            authority,
            clock,
        ],
        signers_seeds,
    )
}
