//! Instruction states definitions.

use crate::{
    find_liquidity_oracle_currency_distribution_program_address, state::DistributionArray,
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
    /// [RS] Authority - liquidity oracle authority to update state.
    UpdateLiquidityOracleAuthority { authority: Pubkey },

    /// Initializes a new currency distribution account.
    ///
    /// Accounts:
    /// [R] Liquidity oracle - off-chain created account.
    /// [RW] CurrencyDistribution - currency distribution account.
    /// [RS] Authority - liquidity oracle authority to update state.
    /// [R] Clock sysvar.
    /// [R] Rent sysvar
    /// [R] System program id
    CreateCurrencyDistribution {
        currency: String,
        value: DistributionArray,
    },

    /// Updates currency distribution account.
    ///
    /// Accounts:
    /// [R] Liquidity oracle - off-chain created account.
    /// [RS] Authority - liquidity oracle authority.
    /// [RW] CurrencyDistribution - currency distribution to update state
    /// [R] Clock sysvar.
    UpdateCurrencyDistribution {
        currency: String,
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
    update_authority: Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*liquidity_oracle, false),
        AccountMeta::new_readonly(*authority, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::UpdateLiquidityOracleAuthority { authority: update_authority },
        accounts,
    )
}

/// Creates 'CreateCurrencyDistribution' instruction.
pub fn create_currency_distribution(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    authority: &Pubkey,
    currency: String,
    distribution_array: DistributionArray,
) -> Instruction {
    let (currency_distribution, _) = find_liquidity_oracle_currency_distribution_program_address(
        program_id,
        liquidity_oracle,
        &currency,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*liquidity_oracle, false),
        AccountMeta::new(currency_distribution, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::CreateCurrencyDistribution {
            currency: currency.to_string(),
            value: distribution_array,
        },
        accounts,
    )
}

pub fn update_currency_distribution(
    program_id: &Pubkey,
    liquidity_oracle: &Pubkey,
    authority: &Pubkey,
    currency: String,
    distribution_array: DistributionArray,
) -> Instruction {
    let (currency_distribution, _) = find_liquidity_oracle_currency_distribution_program_address(
        program_id,
        liquidity_oracle,
        &currency,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*liquidity_oracle, false),
        AccountMeta::new(currency_distribution, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &LiquidityOracleInstruction::UpdateCurrencyDistribution {
            currency: currency.to_string(),
            value: distribution_array,
        },
        accounts,
    )
}
