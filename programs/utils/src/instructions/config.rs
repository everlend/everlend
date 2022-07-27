use anchor_lang::InstructionData;
use eld_config::instruction::Initialize;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

pub fn initialize(
    program_id: &Pubkey,
    config: &Pubkey,
    authority: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*config, true),
            AccountMeta::new(*authority, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: Initialize.data(),
    }
}

