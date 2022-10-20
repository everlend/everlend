use borsh::BorshSerialize;
use solana_program::account_info::AccountInfo;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

/// `global:deposit_liquidity` anchor program instruction
const DEPOSIT_INSTRUCTION: [u8; 8] = [245, 99, 59, 25, 151, 71, 233, 249];
/// `global:unstake_liquidity` anchor program instruction
const REDEEM_INSTRUCTION: [u8; 8] = [133, 140, 234, 156, 146, 93, 40, 244];
/// `global:harvest_liquidity` anchor program instruction
const CLAIM_INSTRUCTION: [u8; 8] = [212, 214, 33, 211, 40, 86, 9, 118];

/// Generates deposit account address
pub fn find_deposit_address(
    program_id: &Pubkey,
    liquidity_pool: &Pubkey,
    user: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            "deposit".as_bytes(),
            &liquidity_pool.to_bytes(),
            &user.to_bytes(),
        ],
        program_id,
    )
}

/// Generates liquidity owner address
pub fn find_owner_address(program_id: &Pubkey, liquidity_pool: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &["nftlendingv2".as_bytes(), &liquidity_pool.to_bytes()],
        program_id,
    )
}

/// Deposit liquidity
#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    program_id: &Pubkey,
    liquidity_pool: AccountInfo<'a>,
    liquidity_owner: AccountInfo<'a>,
    deposit_account: AccountInfo<'a>,
    user: AccountInfo<'a>,
    rent: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct DepositLiquidity {
        instruction: [u8; 8],
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*liquidity_pool.key, false),
            AccountMeta::new(*liquidity_owner.key, false),
            AccountMeta::new(*deposit_account.key, false),
            AccountMeta::new(*user.key, true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(*rent.key, false),
        ],
        data: DepositLiquidity {
            instruction: DEPOSIT_INSTRUCTION,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[liquidity_pool, liquidity_owner, deposit_account, user, rent],
        signers_seeds,
    )
}

/// Redeem liquidity
#[allow(clippy::too_many_arguments)]
pub fn redeem<'a>(
    program_id: &Pubkey,
    liquidity_pool: AccountInfo<'a>,
    deposit_account: AccountInfo<'a>,
    user: AccountInfo<'a>,
    liquidity_owner: AccountInfo<'a>,
    admin: AccountInfo<'a>,
    amount: u64,
    deposit_bump: u8,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct WithdrawFromLendingPool {
        instruction: [u8; 8],
        deposit_bump: u8,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*liquidity_pool.key, false),
            AccountMeta::new(*deposit_account.key, false),
            AccountMeta::new(*user.key, true),
            AccountMeta::new(*liquidity_owner.key, false),
            AccountMeta::new(*admin.key, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: WithdrawFromLendingPool {
            instruction: REDEEM_INSTRUCTION,
            deposit_bump,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            liquidity_pool,
            deposit_account,
            user,
            liquidity_owner,
            admin,
        ],
        signed_seeds,
    )
}

/// Claim rewards
pub fn claim_rewards<'a>(
    program_id: &Pubkey,
    liquidity_pool: AccountInfo<'a>,
    deposit_account: AccountInfo<'a>,
    user: AccountInfo<'a>,
    liquidity_owner: AccountInfo<'a>,
    admin: AccountInfo<'a>,
    deposit_bump: u8,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct HarvestLiquidity {
        instruction: [u8; 8],
        deposit_bump: u8,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*liquidity_pool.key, false),
            AccountMeta::new(*liquidity_owner.key, false),
            AccountMeta::new(*deposit_account.key, false),
            AccountMeta::new(*user.key, true),
            AccountMeta::new(*admin.key, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: HarvestLiquidity {
            instruction: CLAIM_INSTRUCTION,
            deposit_bump,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            liquidity_pool,
            liquidity_owner,
            deposit_account,
            user,
            admin,
        ],
        signed_seeds,
    )
}
