use anchor_lang::InstructionData;
use quarry_mine::instruction::{ClaimRewardsV2, CreateMinerV2, StakeTokens, WithdrawTokens};
use solana_program::account_info::AccountInfo;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::invoke;
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

    invoke(
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

    invoke(
        &instruction,
        &[
            authority,
            miner,
            quarry,
            miner_vault,
            token_account,
            rewarder,
        ],
    )
}

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

    invoke(
        &instruction,
        &[
            authority,
            miner,
            quarry,
            miner_vault,
            token_account,
            rewarder,
        ],
    )
}

/// Claim rewards
pub fn claim_rewards<'a>(
    program_id: &Pubkey,
    mint_wrapper: AccountInfo<'a>,
    minter: AccountInfo<'a>,
    rewards_token_mint: AccountInfo<'a>,
    rewards_token_account: AccountInfo<'a>,
    claim_fee_token_account: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*mint_wrapper.key, true),
            AccountMeta::new(*minter.key, true),
            AccountMeta::new(*rewards_token_mint.key, true),
            AccountMeta::new(*rewards_token_account.key, true),
            AccountMeta::new(*claim_fee_token_account.key, true),
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
            claim_fee_token_account,
        ],
    )
}
