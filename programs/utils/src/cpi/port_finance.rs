use solana_program::{
    account_info::AccountInfo,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_option::COption,
    pubkey::Pubkey,
};

pub fn refresh_reserve<'a>(
    program_id: &Pubkey,
    reserve: AccountInfo<'a>,
    reserve_liquidity_oracle: AccountInfo<'a>,
    clock: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let reserve_key = *reserve.key;
    let mut liquidity_oracle = COption::None;
    let mut account_infos = vec![reserve, clock];

    if reserve_liquidity_oracle.lamports() != 0 {
        liquidity_oracle = COption::Some(*reserve_liquidity_oracle.key);
        account_infos.push(reserve_liquidity_oracle);
    }

    let instruction = port_variable_rate_lending_instructions::instruction::refresh_reserve(
        *program_id,
        reserve_key,
        liquidity_oracle,
    );

    invoke(&instruction, &account_infos)
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
    let instruction =
        port_variable_rate_lending_instructions::instruction::deposit_reserve_liquidity(
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
        &instruction,
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
    let instruction =
        port_variable_rate_lending_instructions::instruction::redeem_reserve_collateral(
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
        &instruction,
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

pub fn create_stake_account<'a>(
    program_id: &Pubkey,
    stake_account: AccountInfo<'a>,
    staking_pool: AccountInfo<'a>,
    stake_account_owner: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let instruction = port_finance_staking::instruction::create_stake_account(
        *program_id,
        *stake_account.key,
        *staking_pool.key,
        *stake_account_owner.key,
    );

    invoke(
        &instruction,
        &[stake_account, staking_pool, stake_account_owner],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn claim_reward<'a>(
    program_id: &Pubkey,
    stake_account_owner: AccountInfo<'a>,
    stake_account: AccountInfo<'a>,
    staking_pool: AccountInfo<'a>,
    reward_token_pool: &Pubkey,
    reward_destination: &Pubkey,
    sub_reward_token_pool: Option<Pubkey>,
    sub_reward_destination: Option<Pubkey>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = port_finance_staking::instruction::claim_reward(
        *program_id,
        *stake_account_owner.key,
        *stake_account.key,
        *staking_pool.key,
        *reward_token_pool,
        sub_reward_token_pool,
        *reward_destination,
        sub_reward_destination,
    );
    invoke_signed(
        &instruction,
        &[stake_account, staking_pool, stake_account_owner],
        signers_seeds,
    )
}

pub fn deposit_staking<'a>(
    program_id: &Pubkey,
    stake_account: AccountInfo<'a>,
    staking_pool: AccountInfo<'a>,
    stake_account_owner: AccountInfo<'a>,
    amount: u64,
) -> Result<(), ProgramError> {
    let ix = port_finance_staking::instruction::deposit(
        *program_id,
        amount,
        *stake_account_owner.key,
        *stake_account.key,
        *staking_pool.key,
    );

    invoke(&ix, &[stake_account_owner, stake_account, staking_pool])
}

pub fn withdraw_staking<'a>(
    program_id: &Pubkey,
    stake_account: AccountInfo<'a>,
    staking_pool: AccountInfo<'a>,
    stake_account_owner: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = port_finance_staking::instruction::withdraw(
        *program_id,
        amount,
        *stake_account_owner.key,
        *stake_account.key,
        *staking_pool.key,
    );

    invoke_signed(
        &ix,
        &[stake_account_owner, stake_account, staking_pool],
        signers_seeds,
    )
}
