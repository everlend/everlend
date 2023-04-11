use crate::cpi::quarry;
use borsh::BorshSerialize;
use solana_program::account_info::AccountInfo;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use std::str::FromStr;

/// `global:stake_primary_miner` anchor program instruction
const STAKE_PRIMARY_INSTRUCTION: [u8; 8] = [72, 59, 23, 242, 117, 178, 129, 138];
/// `global:stake_replica_miner` anchor program instruction
const STAKE_REPLICA_INSTRUCTION: [u8; 8] = [246, 171, 25, 201, 242, 145, 94, 47];
/// `global:unstake_primary_miner` anchor program instruction
const UNSTAKE_PRIMARY_INSTRUCTION: [u8; 8] = [45, 62, 3, 33, 114, 156, 186, 26];
/// `global:unstake_all_replica_miner` anchor program instruction
const UNSTAKE_REPLICA_INSTRUCTION: [u8; 8] = [250, 4, 3, 209, 154, 125, 71, 168];
/// `global:claim_rewards` anchor program instruction
const CLAIM_REWARDS_INSTRUCTION: [u8; 8] = [4, 144, 132, 71, 116, 23, 151, 80];
/// `global:withdraw_tokens` anchor program instruction
const WITHDRAW_INSTRUCTION: [u8; 8] = [2, 4, 225, 61, 19, 182, 106, 170];
/// `global:init_merge_miner_v2` anchor program instruction
const INIT_MERGE_MINER_INSTRUCTION: [u8; 8] = [153, 44, 29, 197, 171, 114, 71, 208];
/// `global:init_miner_v2` anchor program instruction
const INIT_MINER_INSTRUCTION: [u8; 8] = [189, 125, 116, 157, 73, 4, 253, 156];

pub fn staking_program_id() -> Pubkey {
    return Pubkey::from_str("QMMD16kjauP5knBwxNUJRZ1Z5o3deBuFrqVjBVmmqto").unwrap();
}

/// Init miner
#[allow(clippy::too_many_arguments)]
pub fn init_miner<'a>(
    program_id: &Pubkey,
    pool: AccountInfo<'a>,
    merge_miner: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    token_mint: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
    payer: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct InitMiner {
        instruction: [u8; 8],
    }

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*pool.key, false),
            AccountMeta::new_readonly(*merge_miner.key, false),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new_readonly(*rewarder.key, false),
            AccountMeta::new_readonly(*token_mint.key, false),
            AccountMeta::new_readonly(*miner_vault.key, false),
            AccountMeta::new(*payer.key, true),
            AccountMeta::new_readonly(quarry::staking_program_id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: InitMiner {
            instruction: INIT_MINER_INSTRUCTION,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &instruction,
        &[
            pool,
            merge_miner,
            miner,
            quarry,
            rewarder,
            token_mint,
            miner_vault,
            payer,
        ],
        signers_seeds,
    )
}

/// Init merge miner
#[allow(clippy::too_many_arguments)]
pub fn init_merge_miner<'a>(
    program_id: &Pubkey,
    pool: AccountInfo<'a>,
    owner: AccountInfo<'a>,
    merge_miner: AccountInfo<'a>,
    payer: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct InitMergeMiner {
        instruction: [u8; 8],
    }

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*pool.key, false),
            AccountMeta::new(*owner.key, true),
            AccountMeta::new(*merge_miner.key, false),
            AccountMeta::new(*payer.key, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: InitMergeMiner {
            instruction: INIT_MERGE_MINER_INSTRUCTION,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &instruction,
        &[pool, owner, merge_miner, payer],
        signers_seeds,
    )
}

/// Stake primary tokens
#[allow(clippy::too_many_arguments)]
pub fn stake_primary<'a>(
    program_id: &Pubkey,
    mm_owner: AccountInfo<'a>,
    mm_primary_token_account: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    merge_miner: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct StakePrimary {
        instruction: [u8; 8],
    }

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*mm_owner.key, true),
            AccountMeta::new(*mm_primary_token_account.key, false),
            AccountMeta::new(*pool.key, false),
            AccountMeta::new(*merge_miner.key, false),
            AccountMeta::new_readonly(*rewarder.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*miner_vault.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(quarry::staking_program_id(), false),
        ],
        data: StakePrimary {
            instruction: STAKE_PRIMARY_INSTRUCTION,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &instruction,
        &[
            mm_owner,
            mm_primary_token_account,
            pool,
            merge_miner,
            rewarder,
            quarry,
            miner,
            miner_vault,
        ],
        signers_seeds,
    )
}

/// Stake replica tokens
#[allow(clippy::too_many_arguments)]
pub fn stake_replica<'a>(
    program_id: &Pubkey,
    mm_owner: AccountInfo<'a>,
    replica_mint: AccountInfo<'a>,
    replica_mint_token_account: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    merge_miner: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct StakeReplica {
        instruction: [u8; 8],
    }

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*mm_owner.key, true),
            AccountMeta::new(*replica_mint.key, false),
            AccountMeta::new(*replica_mint_token_account.key, false),
            AccountMeta::new(*pool.key, false),
            AccountMeta::new(*merge_miner.key, false),
            AccountMeta::new_readonly(*rewarder.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*miner_vault.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(quarry::staking_program_id(), false),
        ],
        data: StakeReplica {
            instruction: STAKE_REPLICA_INSTRUCTION,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &instruction,
        &[
            mm_owner,
            replica_mint,
            replica_mint_token_account,
            pool,
            merge_miner,
            rewarder,
            quarry,
            miner,
            miner_vault,
        ],
        signers_seeds,
    )
}

/// Stake primary tokens
#[allow(clippy::too_many_arguments)]
pub fn unstake_primary<'a>(
    program_id: &Pubkey,
    mm_owner: AccountInfo<'a>,
    mm_primary_token_account: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    merge_miner: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct UnStakePrimary {
        instruction: [u8; 8],
        amount: u64,
    }

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*mm_owner.key, true),
            AccountMeta::new(*mm_primary_token_account.key, false),
            AccountMeta::new(*pool.key, false),
            AccountMeta::new(*merge_miner.key, false),
            AccountMeta::new_readonly(*rewarder.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*miner_vault.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(quarry::staking_program_id(), false),
        ],
        data: UnStakePrimary {
            instruction: UNSTAKE_PRIMARY_INSTRUCTION,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &instruction,
        &[
            mm_owner,
            mm_primary_token_account,
            pool,
            merge_miner,
            rewarder,
            quarry,
            miner,
            miner_vault,
        ],
        signers_seeds,
    )
}

/// Stake replica tokens
#[allow(clippy::too_many_arguments)]
pub fn unstake_replica<'a>(
    program_id: &Pubkey,
    mm_owner: AccountInfo<'a>,
    replica_mint: AccountInfo<'a>,
    replica_mint_token_account: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    merge_miner: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct UnStakeReplica {
        instruction: [u8; 8],
    }

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*mm_owner.key, true),
            AccountMeta::new(*replica_mint.key, false),
            AccountMeta::new(*replica_mint_token_account.key, false),
            AccountMeta::new(*pool.key, false),
            AccountMeta::new(*merge_miner.key, false),
            AccountMeta::new_readonly(*rewarder.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*miner_vault.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(quarry::staking_program_id(), false),
        ],
        data: UnStakeReplica {
            instruction: UNSTAKE_REPLICA_INSTRUCTION,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &instruction,
        &[
            mm_owner,
            replica_mint,
            replica_mint_token_account,
            pool,
            merge_miner,
            rewarder,
            quarry,
            miner,
            miner_vault,
        ],
        signers_seeds,
    )
}

/// Withdraw tokens
#[allow(clippy::too_many_arguments)]
pub fn withdraw_tokens<'a>(
    program_id: &Pubkey,
    owner: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    merge_miner: AccountInfo<'a>,
    withdraw_mint: AccountInfo<'a>,
    mm_token_account: AccountInfo<'a>,
    token_destination: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct Withdraw {
        instruction: [u8; 8],
        amount: u64,
    }

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*owner.key, true),
            AccountMeta::new(*pool.key, false),
            AccountMeta::new(*merge_miner.key, false),
            AccountMeta::new(*withdraw_mint.key, false),
            AccountMeta::new(*mm_token_account.key, false),
            AccountMeta::new(*token_destination.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: Withdraw {
            instruction: WITHDRAW_INSTRUCTION,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &instruction,
        &[
            owner,
            pool,
            merge_miner,
            withdraw_mint,
            mm_token_account,
            token_destination,
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
    claim_fee_account: AccountInfo<'a>,
    stake_token_account: AccountInfo<'a>,
    pool: AccountInfo<'a>,
    merge_miner: AccountInfo<'a>,
    rewarder: AccountInfo<'a>,
    quarry: AccountInfo<'a>,
    miner: AccountInfo<'a>,
    miner_vault: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct ClaimRewards {
        instruction: [u8; 8],
    }

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*mint_wrapper.key, false),
            AccountMeta::new_readonly(*mint_wrapper_program.key, false),
            AccountMeta::new(*minter.key, false),
            AccountMeta::new(*rewards_token_mint.key, false),
            AccountMeta::new(*rewards_token_account.key, false),
            AccountMeta::new(*claim_fee_account.key, false),
            AccountMeta::new(*stake_token_account.key, false),
            AccountMeta::new(*pool.key, false),
            AccountMeta::new(*merge_miner.key, false),
            AccountMeta::new_readonly(*rewarder.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*miner_vault.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(quarry::staking_program_id(), false),
        ],
        data: ClaimRewards {
            instruction: CLAIM_REWARDS_INSTRUCTION,
        }
        .try_to_vec()?,
    };

    invoke(
        &instruction,
        &[
            mint_wrapper,
            minter,
            rewards_token_mint,
            rewards_token_account,
            claim_fee_account,
            stake_token_account,
            pool,
            merge_miner,
            rewarder,
            quarry,
            miner,
            miner_vault,
        ],
    )
}

pub fn merge_miner_seed() -> String {
    String::from("MergeMiner")
}

/// Generates merge_miner address
#[allow(clippy::too_many_arguments)]
pub fn find_merge_miner_program_address(
    program_id: &Pubkey,
    pool: &Pubkey,
    owner: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            merge_miner_seed().as_bytes().as_ref(),
            &pool.to_bytes().as_ref(),
            &owner.to_bytes().as_ref(),
        ],
        program_id,
    )
}

pub fn pool_seed() -> String {
    String::from("MergePool")
}

/// Generates pool address
#[allow(clippy::too_many_arguments)]
pub fn find_pool_program_address(program_id: &Pubkey, primary_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            pool_seed().as_bytes().as_ref(),
            &primary_mint.to_bytes().as_ref(),
        ],
        program_id,
    )
}

pub fn replica_seed() -> String {
    String::from("ReplicaMint")
}

/// Generates replica_mint address
#[allow(clippy::too_many_arguments)]
pub fn find_replica_mint_program_address(program_id: &Pubkey, pool: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[pool_seed().as_bytes().as_ref(), &pool.to_bytes().as_ref()],
        program_id,
    )
}
