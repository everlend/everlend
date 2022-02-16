use port_variable_rate_lending_instructions::state::Reserve;
use solana_program::{
    account_info::AccountInfo,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
};

pub fn compute_liquidity_amount(
    reserve: AccountInfo,
    collateral_amount: u64,
) -> Result<u64, ProgramError> {
    let reserve = Reserve::unpack(&reserve.data.borrow())?;
    // let collateral_amount = reserve
    //     .collateral_exchange_rate()?
    //     .decimal_liquidity_to_collateral(amount.into())?
    //     .try_round_u64()?;
    // // .liquidity_to_collateral(amount)?;

    let liquidity_amount = reserve
        .collateral_exchange_rate()?
        .collateral_to_liquidity(collateral_amount)?;

    Ok(liquidity_amount)
}

pub fn refresh_reserve<'a>(
    program_id: &Pubkey,
    reserve: AccountInfo<'a>,
    reserve_liquidity_oracle: AccountInfo<'a>,
    clock: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let ix = port_variable_rate_lending_instructions::instruction::refresh_reserve(
        *program_id,
        *reserve.key,
        COption::Some(*reserve_liquidity_oracle.key),
    );

    invoke(&ix, &[reserve, reserve_liquidity_oracle, clock])
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
    let ix = port_variable_rate_lending_instructions::instruction::deposit_reserve_liquidity(
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
    let ix = port_variable_rate_lending_instructions::instruction::redeem_reserve_collateral(
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
            destination_liquidity,
            reserve,
            reserve_collateral_mint,
            reserve_liquidity_supply,
            lending_market,
            lending_market_authority,
            authority,
            clock,
        ],
        signers_seeds,
    )
}
