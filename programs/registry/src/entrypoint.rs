//! Program entrypoint
#![cfg(all(target_arch = "bpf", not(feature = "no-entrypoint")))]
use borsh::BorshDeserialize;
use everlend_utils::EverlendError;
use solana_program::msg;
use solana_program::program_error::PrintProgramError;
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

use crate::instruction::RegistryInstruction;
use crate::instructions::{InitContext, UpdateManagerContext, UpdateRegistryContext};

entrypoint!(program_entrypoint);
fn program_entrypoint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = process_instruction(program_id, accounts, instruction_data) {
        // Catch the error so we can print it
        error.print::<EverlendError>();
        return Err(error);
    }
    Ok(())
}

/// Instruction processing router
fn process_instruction(
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
    }
}
