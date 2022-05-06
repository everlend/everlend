//! Instruction types

use crate::{find_config_program_address, state::{SetRegistryConfigParams, SetPoolConfigParams}};
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
    /// [W] Registry account - uninitialized
    /// [R] Manager
    /// [R] Rent sysvar
    Init,

    /// Set a registry config
    ///
    /// Accounts:
    /// [R] Registry
    /// [W] Registry config
    /// [WS] Manager
    /// [R] Rent sysvar
    /// [R] Sytem program
    SetRegistryConfig {
        /// Set registry config params
        params: SetRegistryConfigParams,
    },

    /// Set pool config
    ///
    /// Accounts:
    /// [R] Registry
    /// [R] General Pool
    /// [W] Pool config
    /// [WS] Manager
    /// [R] Rent sysvar
    /// [R] Sytem program
    SetPoolConfig {
        /// Set pool config params
        params: SetPoolConfigParams,
    },
}

/// Creates 'Init' instruction.
pub fn init(program_id: &Pubkey, registry: &Pubkey, manager: &Pubkey) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*registry, false),
        AccountMeta::new_readonly(*manager, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(*program_id, &RegistryInstruction::Init, accounts)
}

/// Creates 'SetRegistryConfig' instruction.
pub fn set_registry_config(
    program_id: &Pubkey,
    registry: &Pubkey,
    manager: &Pubkey,
    params: SetRegistryConfigParams,
) -> Instruction {
    let (registry_config, _) = find_config_program_address(program_id, registry);

    let accounts = vec![
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new(registry_config, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &RegistryInstruction::SetRegistryConfig { params },
        accounts,
    )
}
