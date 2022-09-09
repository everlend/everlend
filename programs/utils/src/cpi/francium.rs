use borsh::BorshSerialize;
use solana_program::account_info::AccountInfo;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar;

pub fn refresh_reserve<'a>(
    program_id: &Pubkey,
    lending_market: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    clock: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct UpdateLendingPool {
        instruction: u8,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*lending_market.key, false),
            AccountMeta::new_readonly(*reserve.key, false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data: UpdateLendingPool { instruction: 12 }.try_to_vec()?,
    };

    invoke(
        &ix,
        &[lending_market, reserve, clock],
    )
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
            AccountMeta::new(*user_transfer_authority.key, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
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
            AccountMeta::new(*user_transfer_authority.key, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
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
