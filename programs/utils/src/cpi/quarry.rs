use anchor_lang::InstructionData;
use quarry_mine::instruction::{ClaimRewardsV2, CreateMinerV2, StakeTokens, WithdrawTokens};
use quarry_redeemer::instruction::RedeemAllTokens;
use solana_program::account_info::AccountInfo;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use std::str::FromStr;

/// Generates rebalancing seed
pub fn miner_seed() -> String {
    String::from("Miner")
}

pub fn staking_program_id() -> Pubkey {
    return quarry_mine::id();
}

pub fn mine_wrapper_program_id() -> Pubkey {
    return Pubkey::from_str("QMWoBmAyJLAsA1Lh9ugMTw2gciTihncciphzdNzdZYV").unwrap();
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

/// Generates quarry address
pub fn find_quarry_program_address(
    program_id: &Pubkey,
    rewarder: &Pubkey,
    token_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"Quarry".as_ref(),
            &rewarder.to_bytes(),
            &token_mint.to_bytes(),
        ],
        program_id,
    )
}

/// Generates minter address
pub fn find_minter_program_address(
    program_id: &Pubkey,
    mint_wrapper: &Pubkey,
    minter_authority: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            b"MintWrapperMinter".as_ref(),
            &mint_wrapper.to_bytes(),
            &minter_authority.to_bytes(),
        ],
        program_id,
    )
}

/// Create miner
#[allow(clippy::too_many_arguments)]
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
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new_readonly(*rewarder.key, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(*payer.key, true),
            AccountMeta::new_readonly(*token_mint.key, false),
            //User token account
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
#[allow(clippy::too_many_arguments)]
pub fn stake_tokens<'a>(
    program_id: &Pubkey,
    authority: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
    source_token_account: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            //User quarry token account
            AccountMeta::new(*miner_vault.key, false),
            AccountMeta::new(*source_token_account.key, false),
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
            source_token_account,
            rewarder,
        ],
        signers_seeds,
    )
}

/// Withdraw tokens
#[allow(clippy::too_many_arguments)]
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
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(*miner_vault.key, false),
            AccountMeta::new(*token_account.key, false),
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

/// Claim rewards
#[allow(clippy::too_many_arguments)]
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
    signers_seeds: &[&[&[u8]]],
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
            AccountMeta::new(*authority.key, true),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(*quarry_rewarder.key, false),
        ],
        data: ClaimRewardsV2 {}.data(),
    };

    invoke_signed(
        &instruction,
        &[
            mint_wrapper,
            minter,
            rewards_token_mint,
            rewards_token_account,
            rewards_fee_account,
            authority,
            miner,
            quarry,
            quarry_rewarder,
        ],
        signers_seeds,
    )
}

/// Claim rewards
pub fn redeem_all_tokens<'a>(
    quarry_redeemer_program_id: &Pubkey,
    redeemer: AccountInfo<'a>,
    iou_mint: AccountInfo<'a>,
    iou_source: AccountInfo<'a>,
    redemption_vault: AccountInfo<'a>,
    redemption_destination: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = Instruction {
        program_id: *quarry_redeemer_program_id,
        accounts: vec![
            AccountMeta::new(*redeemer.key, false),
            // Authority of the source of the redeemed tokens.
            AccountMeta::new_readonly(*authority.key, true),
            // [Mint] of the IOU token.
            AccountMeta::new(*iou_mint.key, false),
            // Source of the IOU tokens.
            AccountMeta::new(*iou_source.key, false),
            // [TokenAccount] holding the [Redeemer]'s redemption tokens.
            AccountMeta::new(*redemption_vault.key, false),
            // Destination of the IOU tokens.
            AccountMeta::new(*redemption_destination.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: RedeemAllTokens {}.data(),
    };

    invoke_signed(
        &instruction,
        &[
            redeemer,
            authority,
            iou_mint,
            iou_source,
            redemption_vault,
            redemption_destination,
        ],
        signers_seeds,
    )
}
