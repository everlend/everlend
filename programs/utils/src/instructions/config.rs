use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

use crate::AnchorInstruction;

#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub struct ConfigInstruction {
    test: i128
}

pub fn initialize(
    program_id: &Pubkey,
    config: &Pubkey,
    authority: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*config, true),
        AccountMeta::new(*authority, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction::new_with_bytes(
        *program_id,
        &AnchorInstruction::new(b"initialize"),
        accounts,
    )
}

