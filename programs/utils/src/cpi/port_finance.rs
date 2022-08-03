use anchor_lang::Key;
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

    if !reserve_liquidity_oracle.key.eq(&Pubkey::default()) {
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
    rent: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let instruction = port_finance_staking::instruction::create_stake_account(
        *program_id,
        *stake_account.key,
        *staking_pool.key,
        *stake_account_owner.key,
    );

    invoke(
        &instruction,
        &[stake_account, staking_pool, stake_account_owner, rent],
    )
}

pub fn init_obligation<'a>(
    program_id: &Pubkey,
    obligation_account: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    obligation_owner: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    rent: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = port_variable_rate_lending_instructions::instruction::init_obligation(
        *program_id,
        *obligation_account.key,
        *lending_market.key,
        *obligation_owner.key,
    );

    invoke_signed(
        &instruction,
        &[
            obligation_account,
            lending_market,
            obligation_owner,
            clock,
            rent,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_obligation_collateral<'a>(
    program_id: &Pubkey,
    source_collateral: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    deposit_reserve: AccountInfo<'a>,
    obligation: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    obligation_owner: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    option_stake_account: AccountInfo<'a>,
    option_staking_pool: AccountInfo<'a>,
    staking_program: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    collateral_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction =
        port_variable_rate_lending_instructions::instruction::deposit_obligation_collateral(
            *program_id,
            collateral_amount,
            *source_collateral.key,
            *destination_collateral.key,
            *deposit_reserve.key,
            *obligation.key,
            *lending_market.key,
            *obligation_owner.key,
            *user_transfer_authority.key,
            // TODO work with option pool
            Some(*option_stake_account.key),
            Some(*option_staking_pool.key),
        );

    invoke_signed(
        &instruction,
        &[
            source_collateral,
            destination_collateral,
            deposit_reserve,
            obligation,
            lending_market,
            obligation_owner,
            option_stake_account,
            option_staking_pool,
            staking_program,
            lending_market_authority,
            clock,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_obligation_collateral<'a>(
    program_id: &Pubkey,
    source_collateral: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    deposit_reserve: AccountInfo<'a>,
    obligation: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    obligation_owner: AccountInfo<'a>,
    option_stake_account: AccountInfo<'a>,
    option_staking_pool: AccountInfo<'a>,
    staking_program: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    collateral_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction =
        port_variable_rate_lending_instructions::instruction::withdraw_obligation_collateral(
            *program_id,
            collateral_amount,
            *source_collateral.key,
            *destination_collateral.key,
            *deposit_reserve.key,
            *obligation.key,
            *lending_market.key,
            *obligation_owner.key,
            // TODO work with option pool
            Some(*option_stake_account.key),
            Some(*option_staking_pool.key),
        );

    invoke_signed(
        &instruction,
        &[
            source_collateral,
            destination_collateral,
            deposit_reserve,
            obligation,
            lending_market,
            obligation_owner,
            option_stake_account,
            option_staking_pool,
            staking_program,
            lending_market_authority,
            clock,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn refresh_obligation<'a>(
    program_id: &Pubkey,
    obligation: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    clock: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let instruction = port_variable_rate_lending_instructions::instruction::refresh_obligation(
        *program_id,
        obligation.key(),
        vec![reserve.key()],
    );

    invoke(&instruction, &[obligation, reserve, clock])
}

#[allow(clippy::too_many_arguments)]
pub fn claim_reward<'a>(
    program_id: &Pubkey,
    stake_account_owner: AccountInfo<'a>,
    stake_account: AccountInfo<'a>,
    staking_pool: AccountInfo<'a>,
    staking_pool_authority: AccountInfo<'a>,
    reward_token_pool: AccountInfo<'a>,
    reward_destination: AccountInfo<'a>,
    sub_reward_info: Option<(&AccountInfo<'a>, AccountInfo<'a>)>,
    // sub_reward_token_pool: Option<AccountInfo<'a>>,
    // sub_reward_destination: Option<AccountInfo<'a>>,
    clock: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let (sub_reward_token_pool, sub_reward_destination, mut sub_reward_accounts) =
        if let Some(sub_reward) = sub_reward_info {
            (
                Some(sub_reward.0.key()),
                Some(sub_reward.1.key()),
                vec![sub_reward.0.clone(), sub_reward.1.clone()],
            )
        } else {
            (None, None, vec![])
        };

    let instruction = port_finance_staking::instruction::claim_reward(
        *program_id,
        *stake_account_owner.key,
        *stake_account.key,
        *staking_pool.key,
        reward_token_pool.key(),
        sub_reward_token_pool,
        reward_destination.key(),
        sub_reward_destination,
    );

    let mut accounts = vec![
        stake_account_owner,
        stake_account,
        staking_pool,
        reward_token_pool,
        reward_destination,
        staking_pool_authority,
        clock,
        token_program,
    ];

    accounts.append(&mut sub_reward_accounts);

    invoke_signed(&instruction, &accounts, signers_seeds)
}

// pub fn deposit_staking<'a>(
//     program_id: &Pubkey,
//     stake_account: AccountInfo<'a>,
//     staking_pool: AccountInfo<'a>,
//     stake_account_owner: AccountInfo<'a>,
//     clock: AccountInfo<'a>,
//     amount: u64,
//     signers_seeds: &[&[&[u8]]],
// ) -> Result<(), ProgramError> {
//     let ix = port_finance_staking::instruction::deposit(
//         *program_id,
//         amount,
//         *stake_account_owner.key,
//         *stake_account.key,
//         *staking_pool.key,
//     );
//
//     invoke_signed(
//         &ix,
//         &[stake_account_owner, stake_account, staking_pool, clock],
//         signers_seeds,
//     )
// }

// pub fn withdraw_staking<'a>(
//     program_id: &Pubkey,
//     stake_account: AccountInfo<'a>,
//     staking_pool: AccountInfo<'a>,
//     stake_account_owner: AccountInfo<'a>,
//     clock: AccountInfo<'a>,
//     amount: u64,
//     signers_seeds: &[&[&[u8]]],
// ) -> Result<(), ProgramError> {
//     let ix = port_finance_staking::instruction::withdraw(
//         *program_id,
//         amount,
//         *stake_account_owner.key,
//         *stake_account.key,
//         *staking_pool.key,
//     );
//
//     invoke_signed(
//         &ix,
//         &[stake_account_owner, stake_account, staking_pool, clock],
//         signers_seeds,
//     )
// }

