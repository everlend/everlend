use solana_program::program_pack::Pack;
use solana_program::{
    account_info::AccountInfo,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use solend_program::math::TryAdd;

pub fn refresh_reserve<'a>(
    program_id: &Pubkey,
    reserve: AccountInfo<'a>,
    reserve_liquidity_pyth_oracle: AccountInfo<'a>,
    reserve_liquidity_switchboard_oracle: AccountInfo<'a>,
    clock: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let ix = solend_program::instruction::refresh_reserve(
        *program_id,
        *reserve.key,
        *reserve_liquidity_pyth_oracle.key,
        *reserve_liquidity_switchboard_oracle.key,
    );

    invoke(
        &ix,
        &[
            reserve,
            reserve_liquidity_pyth_oracle,
            reserve_liquidity_switchboard_oracle,
            clock,
        ],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    program_id: &Pubkey,
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
    let ix = solend_program::instruction::deposit_reserve_liquidity(
        *program_id,
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
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = solend_program::instruction::redeem_reserve_collateral(
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
            source_collateral,
            reserve,
            reserve_collateral_mint,
            reserve_liquidity_supply,
            lending_market,
            lending_market_authority,
            authority,
            clock,
            destination_liquidity,
        ],
        signers_seeds,
    )
}

pub fn get_real_liquidity_amount(
    reserve: AccountInfo,
    collateral_amount: u64,
) -> Result<u64, ProgramError> {
    let mut reserve = solend_program::state::Reserve::unpack(&reserve.data.borrow())?;

    reserve.redeem_collateral(collateral_amount)
}

pub fn is_deposit_disabled(reserve: AccountInfo) -> Result<bool, ProgramError> {
    let reserve = solend_program::state::Reserve::unpack(&reserve.data.borrow())?;
    let total_asset = reserve
        .liquidity
        .borrowed_amount_wads
        .try_add(solend_program::math::Decimal::from(
            reserve.liquidity.available_amount,
        ))?
        .try_floor_u64()?;
    Ok(reserve.config.deposit_limit == 0 || total_asset >= reserve.config.deposit_limit)
}
