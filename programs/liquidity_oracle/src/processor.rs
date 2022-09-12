//! Program processor.
use borsh::BorshDeserialize;
use solana_program::msg;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::instruction::LiquidityOracleInstruction;
use crate::instructions::{
    CreateTokenDistributionContext, InitContext, UpdateAuthorityContext,
    UpdateTokenDistributionContext,
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
        LiquidityOracleInstruction::CreateTokenDistribution { value } => {
            msg!("LiquidityOracleInstruction: CreateTokenDistribution");
            CreateTokenDistributionContext::new(program_id, accounts)?.process(program_id, value)
        }
        LiquidityOracleInstruction::UpdateTokenDistribution { value } => {
            msg!("LiquidityOracleInstruction: UpdateTokenDistribution");
            UpdateTokenDistributionContext::new(program_id, accounts)?.process(program_id, value)
        }
    }
}
