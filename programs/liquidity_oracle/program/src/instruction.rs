//! Instruction states definitions.

use crate::{
    find_liquidity_oracle_token_distribution_program_address, state::DistributionArray,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

/// Instructions supported by the program.
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum LiquidityOracleInstruction {
    /// Initializes a new liquidity oracle.
    ///
    /// Accounts:
    /// [W] Liquidity oracle - off-chain created account.
    /// [RS] Authority - liquidity oracle authority to update state.
    InitLiquidityOracle,

    /// Updates liquidity oracle.
    ///
    /// Accounts:
    /// [W] Liquidity oracle - account.
    /// [R] Update Authority
    /// [RS] Authority - liquidity oracle authority to update state.
    UpdateLiquidityOracleAuthority,

    /// Initializes a new token distribution account.
    ///
    /// Accounts:
    /// [R]  Liquidity oracle - off-chain created account.
    /// [R]  Token mint account
    /// [RW] TokenDistribution - token distribution account.
    /// [RS] Authority - liquidity oracle authority to update state.
    /// [R]  Clock sysvar.
    /// [R]  Rent sysvar
    /// [R]  System program id
    CreateTokenDistribution {
        value: DistributionArray,
    },

    /// Updates token distribution account.
    ///
    /// Accounts:
    /// [R] Liquidity oracle - off-chain created account.
    /// [R]  Token mint account
    /// [RW] TokenDistribution - token distribution to update state
    /// [RS] Authority - liquidity oracle authority.
    /// [R] Clock sysvar.
    UpdateTokenDistribution {
        value: DistributionArray,
    },
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
    update_authority: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*liquidity_oracle, false),
        AccountMeta::new_readonly(*update_authority, false),
        AccountMeta::new_readonly(*authority, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::UpdateLiquidityOracleAuthority,
        accounts,
    )
}

/// Creates 'CreateTokenDistribution' instruction.
pub fn create_token_distribution(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    authority: &Pubkey,
    token_mint: &Pubkey,
    distribution_array: DistributionArray,
) -> Instruction {
    let (token_distribution, _) = find_liquidity_oracle_token_distribution_program_address(
        program_id,
        liquidity_oracle,
        &token_mint,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*liquidity_oracle, false),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(token_distribution, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::CreateTokenDistribution {
            value: distribution_array,
        },
        accounts,
    )
}

pub fn update_token_distribution(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    authority: &Pubkey,
    token_mint: &Pubkey,
    distribution_array: DistributionArray,
) -> Instruction {
    let (token_distribution, _) = find_liquidity_oracle_token_distribution_program_address(
        program_id,
        liquidity_oracle,
        token_mint,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*liquidity_oracle, false),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(token_distribution, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::UpdateTokenDistribution {
            value: distribution_array,
        },
        accounts,
    )
}
