//! Instruction states definitions.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// Instructions supported by the program.
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum LiquidityOracleInstruction {
    /// Updates liquidity oracle.
    ///
    /// Accounts:
    /// [W] Liquidity oracle - account.
    /// [RS] Authority - liquidity oracle authority to update state.
    UpdateLiquidityOracleAuthority { value: [u8; 32] },

    /// Initializes a new liquidity oracle.
    ///
    /// Accounts:
    /// [W] Liquidity oracle - off-chain created account.
    /// [RS] Authority - liquidity oracle authority to update state.
    InitLiquidityOracle,
}

/// Creates 'InitLiquidityOracle' instruction.
pub fn init_liquidity_oracle(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    authority: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*liquidity_oracle, false),
        AccountMeta::new_readonly(*authority, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::InitLiquidityOracle,
        accounts,
    )
}

/// Creates 'UpdateLiquidityOracleAuthority' instruction.
pub fn update_liquidity_oracle_authority(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    authority: &Pubkey,
    value: [u8; 32],
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*liquidity_oracle, false),
        AccountMeta::new_readonly(*authority, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::UpdateLiquidityOracleAuthority { value },
        accounts,
    )
}
