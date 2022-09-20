//! Program processor.
use borsh::BorshDeserialize;
use solana_program::msg;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instruction::LiquidityOracleInstruction;
use crate::instructions::{
    CreateTokenOracleContext, InitContext, MigrateContext, UpdateAuthorityContext,
    UpdateLiquidityDistributionContext, UpdateReserveRatesContext,
};

/// Instruction processing router.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = LiquidityOracleInstruction::try_from_slice(input)?;

    match instruction {
        LiquidityOracleInstruction::InitLiquidityOracle => {
            msg!("LiquidityOracleInstruction: InitLiquidityOracle");
            InitContext::new(program_id, accounts)?.process(program_id)
        }

        LiquidityOracleInstruction::UpdateLiquidityOracleAuthority => {
            msg!("LiquidityOracleInstruction: UpdateLiquidityOracleAuthority");
            UpdateAuthorityContext::new(program_id, accounts)?.process(program_id)
        }

        LiquidityOracleInstruction::CreateTokenOracle { value } => {
            msg!("LiquidityOracleInstruction: CreateTokenOracle");
            CreateTokenOracleContext::new(program_id, accounts)?.process(program_id, value)
        }

        LiquidityOracleInstruction::UpdateLiquidityDistribution { value } => {
            msg!("LiquidityOracleInstruction: UpdateLiquidityDistribution");
            UpdateLiquidityDistributionContext::new(program_id, accounts)?
                .process(program_id, value)
        }

        LiquidityOracleInstruction::UpdateReserveRates { value } => {
            msg!("LiquidityOracleInstruction: UpdateReserveRates");
            UpdateReserveRatesContext::new(program_id, accounts)?.process(program_id, value)
        }

        LiquidityOracleInstruction::Migrate => {
            msg!("LiquidityOracleInstruction: Migrate");
            MigrateContext::new(program_id, accounts)?.process(program_id)
        }
    }
}
