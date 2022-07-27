use anchor_lang::InstructionData;
use eld_rewards::instruction::FillVault;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::{
    account_info::AccountInfo, program::invoke_signed, program_error::ProgramError, pubkey::Pubkey,
};

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
) -> Result<(), ProgramError> {
    let ix = Instruction {
        program_id: *program_id,
        data: FillVault { amount }.data(),
        accounts: vec![
            AccountMeta::new_readonly(*config.key, false),
            AccountMeta::new(*reward_pool.key, false),
            AccountMeta::new_readonly(*reward_mint.key, false),
            AccountMeta::new(*vault.key, false),
            AccountMeta::new(*fee_account.key, false),
            AccountMeta::new(*authority.key, true),
            AccountMeta::new(*from.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
    };

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
