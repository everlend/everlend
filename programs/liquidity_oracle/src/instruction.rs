//! Instruction states definitions.
use crate::{find_token_oracle_program_address, state::DistributionArray};
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

    /// Initializes a new token oracle account.
    ///
    /// Accounts:
    /// [R]  Liquidity oracle - off-chain created account.
    /// [R]  Token mint account
    /// [RW] Token oracle - token distribution account.
    /// [RS] Authority - liquidity oracle authority to update state.
    /// [R]  Clock sysvar.
    /// [R]  Rent sysvar
    /// [R]  System program id
    CreateTokenOracle { value: DistributionArray },

    /// Updates token distribution account.
    ///
    /// Accounts:
    /// [R] Liquidity oracle - off-chain created account.
    /// [R]  Token mint account
    /// [RW] TokenOracle - to update state
    /// [RS] Authority - liquidity oracle authority.
    /// [R] Clock sysvar.
    UpdateLiquidityDistribution { value: DistributionArray },

    /// Updates money market reserve rates
    ///
    /// Accounts:
    /// [R] Liquidity oracle - off-chain created account.
    /// [R]  Token mint account
    /// [W] TokenOracle - to update state
    /// [RS] Authority - liquidity oracle authority.
    /// [R] Clock sysvar.
    UpdateReserveRates { value: DistributionArray },

    /// Migrate token distribution account.
    ///
    /// Accounts:
    /// [R] Liquidity oracle - off-chain created account.
    /// [R]  Token mint account
    /// [W] TokenOracle - to update state
    /// [RS] Authority - liquidity oracle authority.
    /// [R]  Rent sysvar
    /// [R]  System program id
    Migrate,
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

/// Creates 'CreateTokenOracle' instruction.
pub fn create_token_oracle(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    authority: &Pubkey,
    token_mint: &Pubkey,
    distribution_array: DistributionArray,
) -> Instruction {
    let (token_oracle, _) =
        find_token_oracle_program_address(program_id, liquidity_oracle, token_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*liquidity_oracle, false),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(token_oracle, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::CreateTokenOracle {
            value: distribution_array,
        },
        accounts,
    )
}

pub fn update_liquidity_distribution(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    authority: &Pubkey,
    token_mint: &Pubkey,
    distribution_array: DistributionArray,
) -> Instruction {
    let (token_oracle, _) =
        find_token_oracle_program_address(program_id, liquidity_oracle, token_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*liquidity_oracle, false),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(token_oracle, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::UpdateLiquidityDistribution {
            value: distribution_array,
        },
        accounts,
    )
}

pub fn update_reserve_rates(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    authority: &Pubkey,
    token_mint: &Pubkey,
    reserve_rates: DistributionArray,
) -> Instruction {
    let (token_oracle, _) =
        find_token_oracle_program_address(program_id, liquidity_oracle, token_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*liquidity_oracle, false),
        AccountMeta::new_readonly(*token_mint, false),
        AccountMeta::new(token_oracle, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::UpdateReserveRates {
            value: reserve_rates,
        },
        accounts,
    )
}
