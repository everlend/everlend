use anchor_lang::InstructionData;
use quarry_mine::instruction::{ClaimRewardsV2, CreateMinerV2, StakeTokens, WithdrawTokens};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

/// Generates rebalancing seed
pub fn miner_seed() -> String {
    String::from("Miner")
}

/// Generates internal mining address
pub fn find_miner_program_address(
    program_id: &Pubkey,
    quarry: &Pubkey,
    authority: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            miner_seed().as_bytes(),
            &quarry.to_bytes(),
            &authority.to_bytes(),
        ],
        program_id,
    )
}

/// Create miner
pub fn create_miner<'a>(
    program_id: &Pubkey,
    authority: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    payer: AccountInfo<'a>,
    token_mint: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new_readonly(*miner.key, false),
            AccountMeta::new_readonly(*quarry.key, false),
            AccountMeta::new_readonly(*rewarder.key, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(*payer.key, false),
            AccountMeta::new_readonly(*token_mint.key, false),
            AccountMeta::new_readonly(*miner_vault.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: CreateMinerV2.data(),
    };

    invoke_signed(
        &instruction,
        &[
            authority,
            miner,
            quarry,
            rewarder,
            payer,
            token_mint,
            miner_vault,
        ],
        signers_seeds,
    )
}

/// Stake tokens
pub fn stake_tokens<'a>(
    program_id: &Pubkey,
    authority: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new_readonly(*miner.key, false),
            AccountMeta::new_readonly(*quarry.key, false),
            AccountMeta::new_readonly(*miner_vault.key, false),
            AccountMeta::new_readonly(*token_account.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(*rewarder.key, false),
        ],
        data: StakeTokens { amount }.data(),
    };

    invoke_signed(
        &instruction,
        &[
            authority,
            miner,
            quarry,
            miner_vault,
            token_account,
            rewarder,
        ],
        signers_seeds,
    )
}

// TODO add signer seeds
/// Withdraw tokens
pub fn withdraw_tokens<'a>(
    program_id: &Pubkey,
    authority: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
    token_account: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new_readonly(*miner.key, false),
            AccountMeta::new_readonly(*quarry.key, false),
            AccountMeta::new_readonly(*miner_vault.key, false),
            AccountMeta::new_readonly(*token_account.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(*rewarder.key, false),
        ],
        data: WithdrawTokens { amount }.data(),
    };

    invoke_signed(
        &instruction,
        &[
            authority,
            miner,
            quarry,
            miner_vault,
            token_account,
            rewarder,
        ],
        signers_seeds,
    )
}

// TODO Check Instruction looks like uncomplited
/// Claim rewards
pub fn claim_rewards<'a>(
    program_id: &Pubkey,
    mint_wrapper: AccountInfo<'a>,
    mint_wrapper_program: AccountInfo<'a>,
    minter: AccountInfo<'a>,
    rewards_token_mint: AccountInfo<'a>,
    rewards_token_account: AccountInfo<'a>,
    rewards_fee_account: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    quarry_rewarder: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*mint_wrapper.key, false),
            AccountMeta::new_readonly(*mint_wrapper_program.key, false),
            AccountMeta::new(*minter.key, false),
            AccountMeta::new(*rewards_token_mint.key, false),
            AccountMeta::new(*rewards_token_account.key, false),
            AccountMeta::new(*rewards_fee_account.key, false),
            AccountMeta::new(*authority.key, false),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(*quarry_rewarder.key, false),
        ],
        data: ClaimRewardsV2 {}.data(),
    };

    invoke(
        &instruction,
        &[
            mint_wrapper,
            minter,
            rewards_token_mint,
            rewards_token_account,
            rewards_fee_account,
            miner,
            quarry,
            quarry_rewarder,
        ],
    )
}
