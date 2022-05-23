use anchor_lang::InstructionData;
use quarry_mine::instruction::{ClaimRewardsV2, CreateMinerV2, StakeTokens};
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

/// Accounts for [quarry_mine::create_miner].
// pub struct CreateMiner<'info> {
//     /// Authority of the [Miner].
//     pub authority: Signer<'info>,
//     /// [Miner] to be created.
//     #[account(init, seeds = [b"Miner".as_ref (), quarry.key ().to_bytes ().as_ref (), authority.key ().to_bytes ().as_ref ()], bump, payer = payer, space = 8 + Miner::LEN)]
//     pub miner: Box<Account<'info, Miner>>,
//     /// [Quarry] to create a [Miner] for.
//     #[account(mut)]
//     pub quarry: Box<Account<'info, Quarry>>,
//     /// [Rewarder].
//     pub rewarder: Box<Account<'info, Rewarder>>,
//     /// System program.
//     pub system_program: Program<'info, System>,
//     /// Payer of [Miner] creation.
//     #[account(mut)]
//     pub payer: Signer<'info>,
//     /// [Mint] of the token to create a [Quarry] for.
//     pub token_mint: Account<'info, Mint>,
//     /// [TokenAccount] holding the token [Mint].
//     pub miner_vault: Account<'info, TokenAccount>,
//     /// SPL Token program.
//     pub token_program: Program<'info, Token>,
// }
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
    let ix = Instruction {
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
        &ix,
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

/// Staking accounts
///
/// This accounts struct is always used in the context of the user authority
/// staking into an account. This is NEVER used by an admin.
///
/// Validation should be extremely conservative.
// pub struct UserStake<'info> {
//     /// Miner authority (i.e. the user).
//     pub authority: Signer<'info>,
//     /// Miner.
//     #[account(mut)]
//     pub miner: Account<'info, Miner>,
//     /// Quarry to claim from.
//     #[account(mut)]
//     pub quarry: Account<'info, Quarry>,
//     /// Vault of the miner.
//     #[account(mut)]
//     pub miner_vault: Account<'info, TokenAccount>,
//     /// User's staked token account
//     #[account(mut)]
//     pub token_account: Account<'info, TokenAccount>,
//     /// Token program
//     pub token_program: Program<'info, Token>,
//     /// Rewarder
//     pub rewarder: Account<'info, Rewarder>,
// }
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
    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new_readonly(*miner.key, false),
            AccountMeta::new_readonly(*quarry.key, false),
            AccountMeta::new_readonly(*miner_vault.key, false),
            AccountMeta::new_readonly(*token_account.key, false),
            AccountMeta::new_readonly(*rewarder.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: StakeTokens { amount }.data(),
    };

    invoke(
        &ix,
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

/////ClaimRewardsV2 accounts
// pub struct ClaimRewardsV2<'info> {
//     /// Mint wrapper.
//     #[account(mut)]
//     pub mint_wrapper: Box<Account<'info, quarry_mint_wrapper::MintWrapper>>,
//     /// Mint wrapper program.
//     pub mint_wrapper_program:
//     Program<'info, quarry_mint_wrapper::program::QuarryMintWrapper>,
//     /// [quarry_mint_wrapper::Minter] information.
//     #[account(mut)]
//     pub minter: Box<Account<'info, quarry_mint_wrapper::Minter>>,
//     /// Mint of the rewards token.
//     #[account(mut)]
//     pub rewards_token_mint: Box<Account<'info, Mint>>,
//     /// Account to claim rewards for.
//     #[account(mut)]
//     pub rewards_token_account: Box<Account<'info, TokenAccount>>,
//     /// Account to send claim fees to.
//     #[account(mut)]
//     pub claim_fee_token_account: Box<Account<'info, TokenAccount>>,
//     /// Claim accounts
//     pub claim: UserClaimV2<'info>,
// }
// pub fn claim_rewards<'a>(
//     program_id: &Pubkey,
//     mint_wrapper: AccountInfo<'a>,
//     mint_wrapper_program: AccountInfo<'a>,
//     minter: AccountInfo<'a>,
//     rewards_token_mint: AccountInfo<'a>,
//     rewards_token_account: AccountInfo<'a>,
//     claim_fee_token_account: AccountInfo<'a>,
//     authority: AccountInfo<'a>,
//     miner: AccountInfo<'a>,
//     quarry: AccountInfo<'a>,
//     rewarder: AccountInfo<'a>,
//     amount: u64,
// ) -> Result<(), ProgramError> {
//     let ix = Instruction {
//         program_id: *program_id,
//         accounts: vec![
//             AccountMeta::new_readonly(*authority.key, true),
//             AccountMeta::new_readonly(*miner.key, false),
//             AccountMeta::new_readonly(*quarry.key, false),
//             AccountMeta::new_readonly(*miner_vault.key, false),
//             AccountMeta::new_readonly(*token_account.key, false),
//             AccountMeta::new_readonly(*rewarder.key, false),
//             AccountMeta::new_readonly(spl_token::id(), false),
//         ],
//         data: ClaimRewardsV2 { amount }.data(),
//     };
//
//     invoke(&ix, &[])
// }
