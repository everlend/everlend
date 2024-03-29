//! Instruction types

use crate::instructions::{UpdateRegistryData, UpdateRegistryMarketsData};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

/// Instructions supported by the program
#[allow(clippy::large_enum_variant)]
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum RegistryInstruction {
    /// Initializes a new registry
    ///
    /// Accounts:
    /// [WS] Registry account - uninitialized
    /// [WS] Manager
    /// [R] System program
    /// [R] Rent sysvar
    Init,

    /// Update manager
    ///
    /// Accounts:
    /// [W] Registry
    /// [S] Old manager
    /// [S] New manager
    ///
    UpdateManager,

    /// Set a registry config
    ///
    /// Accounts:
    /// [W] Registry
    /// [S] Manager
    UpdateRegistry {
        /// Registry data to update
        data: UpdateRegistryData,
    },

    /// Update registry markets
    ///
    /// Accounts:
    /// [W] Registry
    /// [S] Manager
    UpdateRegistryMarkets {
        /// MoneyMarkets data to update
        data: UpdateRegistryMarketsData,
    },
}

/// Creates 'Init' instruction.
pub fn init(program_id: &Pubkey, registry: &Pubkey, manager: &Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*registry, true),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(*program_id, &RegistryInstruction::Init, accounts)
}

/// Creates 'UpdateManager' instruction.
#[allow(clippy::too_many_arguments)]
pub fn update_manager(
    program_id: &Pubkey,
    registry: &Pubkey,
    manager: &Pubkey,
    new_manager: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*registry, false),
        AccountMeta::new_readonly(*manager, true),
        AccountMeta::new_readonly(*new_manager, true),
    ];

    Instruction::new_with_borsh(*program_id, &RegistryInstruction::UpdateManager, accounts)
}

/// Creates 'UpdateRegistry' instruction.
pub fn update_registry(
    program_id: &Pubkey,
    registry: &Pubkey,
    manager: &Pubkey,
    data: UpdateRegistryData,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*registry, false),
        AccountMeta::new_readonly(*manager, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &RegistryInstruction::UpdateRegistry { data },
        accounts,
    )
}

/// Creates 'UpdateRegistryMarkets' instruction.
pub fn update_registry_markets(
    program_id: &Pubkey,
    registry: &Pubkey,
    manager: &Pubkey,
    data: UpdateRegistryMarketsData,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*registry, false),
        AccountMeta::new_readonly(*manager, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &RegistryInstruction::UpdateRegistryMarkets { data },
        accounts,
    )
}
