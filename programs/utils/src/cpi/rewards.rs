use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
};

use crate::instructions::rewards;

#[allow(clippy::too_many_arguments)]
pub fn initialize_mining<'a>(
    program_id: &Pubkey,
    config: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    user: AccountInfo<'a>,
    payer: AccountInfo<'a>,
    system_program: AccountInfo<'a>,
    rent: AccountInfo<'a>,
) -> ProgramResult {
    let ix = rewards::initialize_mining(
        program_id,
        config.key,
        reward_pool.key,
        mining.key,
        user.key,
        payer.key,
    );

    invoke(
        &ix,
        &[
            config,
            reward_pool,
            mining,
            user,
            payer,
            system_program,
            rent,
        ],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_mining<'a>(
    program_id: &Pubkey,
    config: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    user: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let ix = rewards::deposit_mining(
        program_id,
        config.key,
        reward_pool.key,
        mining.key,
        user.key,
        deposit_authority.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[config, reward_pool, mining, user, deposit_authority],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_mining<'a>(
    program_id: &Pubkey,
    config: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    user: AccountInfo<'a>,
    deposit_authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let ix = rewards::withdraw_mining(
        program_id,
        config.key,
        reward_pool.key,
        mining.key,
        user.key,
        deposit_authority.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[config, reward_pool, mining, user, deposit_authority],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn fill_vault<'a>(
    program_id: &Pubkey,
    config: AccountInfo<'a>,
    reward_pool: AccountInfo<'a>,
    reward_mint: AccountInfo<'a>,
    fee_account: AccountInfo<'a>,
    vault: AccountInfo<'a>,
    from: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let ix = rewards::fill_vault(
        program_id,
        config.key,
        reward_pool.key,
        reward_mint.key,
        vault.key,
        fee_account.key,
        authority.key,
        from.key,
        amount,
    );

    invoke_signed(
        &ix,
        &[
            config,
            reward_pool,
            reward_mint,
            vault,
            fee_account,
            from,
            authority,
        ],
        signers_seeds,
    )
}
