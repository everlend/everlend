use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
};

use crate::instructions::rewards;

pub fn initialize_mining<'a>(
    program_id: AccountInfo<'a>,
    config: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    user: AccountInfo<'a>,
    payer: AccountInfo<'a>,
    system_program: AccountInfo<'a>,
    rent:  AccountInfo<'a>,
) -> ProgramResult {
    let ix = rewards::initialize_mining(program_id.key, config.key, reward_pool.key, mining.key, user.key, payer.key);

    invoke(&ix, &[
        config,
        reward_pool,
        mining,
        user,
        payer,
        system_program,
        rent,
    ])
}

pub fn deposit_mining<'a>(
    program_id: AccountInfo<'a>,
    config: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    user: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let ix = rewards::deposit_mining(program_id.key, config.key, reward_pool.key, mining.key, user.key, deposit_authority.key, amount);

    invoke_signed(&ix, &[
        config,
        reward_pool,
        mining,
        user,
        deposit_authority,
    ], signers_seeds)
}

pub fn withdraw_mining<'a>(
    program_id: AccountInfo<'a>,
    config: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    user: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let ix = rewards::withdraw_mining(program_id.key, config.key, reward_pool.key, mining.key, user.key, deposit_authority.key, amount);

    invoke_signed(&ix, &[
        config,
        reward_pool,
        mining,
        user,
        deposit_authority,
    ], signers_seeds)
}