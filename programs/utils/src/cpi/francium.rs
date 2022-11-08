use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Slot;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar;
use std::str::FromStr;

pub const FRANCIUM_REWARD_SEED: &str = "francium_reward";

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct FarmingPool {
    pub version: u8,
    pub is_dual_rewards: u8,
    pub admin: Pubkey,
    pub pool_authority: Pubkey,
    pub token_program_id: Pubkey,

    // staked_token
    pub staked_token_mint: Pubkey,
    pub staked_token_account: Pubkey,

    // reward_token
    pub rewards_token_mint: Pubkey,
    pub rewards_token_account: Pubkey,

    // reward_token_b
    pub rewards_token_mint_b: Pubkey,
    pub rewards_token_account_b: Pubkey,

    // rewards config
    pub pool_stake_cap: u64,
    pub user_stake_cap: u64,
    // rewards a
    pub rewards_start_slot: Slot,
    pub rewards_end_slot: Slot,
    pub rewards_per_day: u64,

    // rewards b
    pub rewards_start_slot_b: Slot,
    pub rewards_end_slot_b: Slot,
    pub rewards_per_day_b: u64,

    pub total_staked_amount: u64,
    pub last_update_slot: Slot,

    pub accumulated_rewards_per_share: u128,
    pub accumulated_rewards_per_share_b: u128,
    pub padding: [u8; 128],
}

pub fn refresh_reserve(program_id: &Pubkey, reserve: AccountInfo) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct UpdateLendingPool {
        instruction: u8,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new(*reserve.key, false)],
        data: UpdateLendingPool { instruction: 17 }.try_to_vec()?,
    };

    invoke(&ix, &[reserve])
}

#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    program_id: &Pubkey,
    source_liquidity: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    reserve_collateral_mint: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct DepositToLendingPool {
        instruction: u8,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*source_liquidity.key, false),
            AccountMeta::new(*destination_collateral.key, false),
            AccountMeta::new(*reserve.key, false),
            AccountMeta::new(*reserve_liquidity_supply.key, false),
            AccountMeta::new(*reserve_collateral_mint.key, false),
            AccountMeta::new_readonly(*lending_market.key, false),
            AccountMeta::new_readonly(*lending_market_authority.key, false),
            AccountMeta::new_readonly(*user_transfer_authority.key, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: DepositToLendingPool {
            instruction: 4,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            source_liquidity,
            destination_collateral,
            reserve,
            reserve_liquidity_supply,
            reserve_collateral_mint,
            lending_market,
            lending_market_authority,
            user_transfer_authority,
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
    user_transfer_authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct WithdrawFromLendingPool {
        instruction: u8,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*source_collateral.key, false),
            AccountMeta::new(*destination_liquidity.key, false),
            AccountMeta::new(*reserve.key, false),
            AccountMeta::new(*reserve_collateral_mint.key, false),
            AccountMeta::new(*reserve_liquidity_supply.key, false),
            AccountMeta::new_readonly(*lending_market.key, false),
            AccountMeta::new_readonly(*lending_market_authority.key, false),
            AccountMeta::new_readonly(*user_transfer_authority.key, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: WithdrawFromLendingPool {
            instruction: 5,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            source_collateral,
            destination_liquidity,
            reserve,
            reserve_collateral_mint,
            reserve_liquidity_supply,
            lending_market,
            lending_market_authority,
            user_transfer_authority,
            clock,
        ],
        signed_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn stake<'a>(
    program_id: &Pubkey,
    user_wallet: AccountInfo<'a>,
    user_farming: AccountInfo<'a>,
    user_stake_token: AccountInfo<'a>,
    user_reward_a: AccountInfo<'a>,
    user_reward_b: AccountInfo<'a>,
    farming_pool: AccountInfo<'a>,
    farming_pool_authority: AccountInfo<'a>,
    pool_stake_token: AccountInfo<'a>,
    pool_reward_a: AccountInfo<'a>,
    pool_reward_b: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct Stake {
        instruction: u8,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*user_wallet.key, true),
            AccountMeta::new(*user_farming.key, false),
            AccountMeta::new(*user_stake_token.key, false),
            AccountMeta::new(*user_reward_a.key, false),
            AccountMeta::new(*user_reward_b.key, false),
            AccountMeta::new(*farming_pool.key, false),
            AccountMeta::new_readonly(*farming_pool_authority.key, false),
            AccountMeta::new(*pool_stake_token.key, true),
            AccountMeta::new(*pool_reward_a.key, true),
            AccountMeta::new(*pool_reward_b.key, true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data: Stake {
            instruction: 3,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            user_wallet,
            user_farming,
            user_stake_token,
            user_reward_a,
            user_reward_b,
            farming_pool,
            farming_pool_authority,
            pool_stake_token,
            pool_reward_a,
            pool_reward_b,
            clock,
        ],
        signed_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn unstake<'a>(
    program_id: &Pubkey,
    user_wallet: AccountInfo<'a>,
    user_farming: AccountInfo<'a>,
    user_stake_token: AccountInfo<'a>,
    user_reward_a: AccountInfo<'a>,
    user_reward_b: AccountInfo<'a>,
    farming_pool: AccountInfo<'a>,
    farming_pool_authority: AccountInfo<'a>,
    pool_stake_token: AccountInfo<'a>,
    pool_reward_a: AccountInfo<'a>,
    pool_reward_b: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct Unstake {
        instruction: u8,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*user_wallet.key, true),
            AccountMeta::new(*user_farming.key, false),
            AccountMeta::new(*user_stake_token.key, false),
            AccountMeta::new(*user_reward_a.key, false),
            AccountMeta::new(*user_reward_b.key, false),
            AccountMeta::new(*farming_pool.key, false),
            AccountMeta::new_readonly(*farming_pool_authority.key, false),
            AccountMeta::new(*pool_stake_token.key, true),
            AccountMeta::new(*pool_reward_a.key, true),
            AccountMeta::new(*pool_reward_b.key, true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data: Unstake {
            instruction: 4,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            user_wallet,
            user_farming,
            user_stake_token,
            user_reward_a,
            user_reward_b,
            farming_pool,
            farming_pool_authority,
            pool_stake_token,
            pool_reward_a,
            pool_reward_b,
            clock,
        ],
        signed_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn init_farming_user<'a>(
    program_id: &Pubkey,
    user_wallet: AccountInfo<'a>,
    user_farming: AccountInfo<'a>,
    farming_pool: AccountInfo<'a>,
    user_stake_token: AccountInfo<'a>,
    user_reward_a: AccountInfo<'a>,
    user_reward_b: AccountInfo<'a>,
    system_program: AccountInfo<'a>,
    rent_info: AccountInfo<'a>,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct Init {
        instruction: u8,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*user_wallet.key, true),
            AccountMeta::new(*user_farming.key, false),
            AccountMeta::new(*farming_pool.key, false),
            AccountMeta::new(*user_stake_token.key, false),
            AccountMeta::new(*user_reward_a.key, false),
            AccountMeta::new(*user_reward_b.key, false),
            AccountMeta::new_readonly(*system_program.key, false),
            AccountMeta::new_readonly(*rent_info.key, false),
        ],
        data: Init { instruction: 1 }.try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            user_wallet,
            user_farming,
            farming_pool,
            user_stake_token,
            user_reward_a,
            user_reward_b,
        ],
        signed_seeds,
    )
}

pub fn get_staking_program_id() -> Pubkey {
    Pubkey::from_str("3Katmm9dhvLQijAvomteYMo6rfVbY5NaCRNq9ZBqBgr6").unwrap()
}

pub fn find_user_farming_address(
    depositor_authority: &Pubkey,
    farming_pool: &Pubkey,
    user_stake_token_account: &Pubkey,
) -> Pubkey {
    let (user_farming, _) = Pubkey::find_program_address(
        &[
            depositor_authority.as_ref(),
            farming_pool.as_ref(),
            user_stake_token_account.as_ref(),
        ],
        &get_staking_program_id(),
    );
    user_farming
}
