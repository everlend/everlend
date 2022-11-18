use solana_program::account_info::AccountInfo;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

pub fn refresh_reserve<'a>(
    program_id: &Pubkey,
    reserve: AccountInfo<'a>,
    reserve_liquidity_oracle: AccountInfo<'a>,
    clock: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let ix = tulipv2_sdk_lending::instruction::refresh_reserve(
        *program_id,
        *reserve.key,
        *reserve_liquidity_oracle.key,
    );

    invoke(&ix, &[reserve, reserve_liquidity_oracle, clock])
}

#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    program_id: &Pubkey,
    source_liquidity: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    reserve_collateral_mint: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = tulipv2_sdk_lending::instruction::deposit_reserve_liquidity(
        *program_id,
        amount,
        *source_liquidity.key,
        *destination_collateral.key,
        *reserve.key,
        *reserve_liquidity_supply.key,
        *reserve_collateral_mint.key,
        *lending_market.key,
        *user_transfer_authority.key,
    );

    invoke_signed(
        &ix,
        &[
            source_liquidity.clone(),
            destination_collateral.clone(),
            reserve.clone(),
            reserve_liquidity_supply.clone(),
            reserve_collateral_mint.clone(),
            lending_market.clone(),
            lending_market_authority.clone(),
            user_transfer_authority.clone(),
            clock.clone(),
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn redeem<'a>(
    program_id: &Pubkey,
    source_collateral: AccountInfo<'a>,
    destination_liquidity: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    reserve_collateral_mint: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = tulipv2_sdk_lending::instruction::redeem_reserve_collateral(
        *program_id,
        amount,
        *source_collateral.key,
        *destination_liquidity.key,
        *reserve.key,
        *reserve_collateral_mint.key,
        *reserve_liquidity_supply.key,
        *lending_market.key,
        *authority.key,
    );

    invoke_signed(
        &ix,
        &[
            source_collateral.clone(),
            destination_liquidity.clone(),
            reserve.clone(),
            reserve_liquidity_supply.clone(),
            reserve_collateral_mint.clone(),
            lending_market.clone(),
            lending_market_authority.clone(),
            authority.clone(),
            clock.clone(),
        ],
        signed_seeds,
    )
}

pub fn get_real_liquidity_amount(
    reserve: AccountInfo,
    collateral_amount: u64,
) -> Result<u64, ProgramError> {
    let mut reserve =
        tulipv2_sdk_common::lending::reserve::Reserve::unpack(&reserve.data.borrow())?;

    reserve.redeem_collateral(collateral_amount)
}
