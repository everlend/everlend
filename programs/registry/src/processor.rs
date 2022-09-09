//! Program processor
use borsh::BorshDeserialize;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

use crate::instruction::RegistryInstruction;
use crate::instructions::{
    InitContext, UpdateManagerContext, UpdateRegistryContext, UpdateRegistryMarketsContext,
};

/// Instruction processing router
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = RegistryInstruction::try_from_slice(input)?;

    match instruction {
        RegistryInstruction::Init => {
            msg!("RegistryInstruction: Init");
            InitContext::new(program_id, accounts)?.process(program_id)
        }

        RegistryInstruction::UpdateManager => {
            msg!("RegistryInstruction: UpdateManager");
            UpdateManagerContext::new(program_id, accounts)?.process(program_id)
        }

        RegistryInstruction::UpdateRegistry { data } => {
            msg!("RegistryInstruction: UpdateRegistry");
            UpdateRegistryContext::new(program_id, accounts)?.process(program_id, data)
        }

        RegistryInstruction::UpdateRegistryMarkets { data } => {
            msg!("RegistryInstruction: UpdateRegistryMarkets");
            UpdateRegistryMarketsContext::new(program_id, accounts)?.process(program_id, data)
        }
    }
}
