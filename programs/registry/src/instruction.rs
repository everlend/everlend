//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

use crate::state::PoolMarketsConfig;
use crate::{find_config_program_address, state::SetRegistryConfigParams};

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
    /// [R] General pool market
    /// [R] Income pool market
    /// [R] ULP pool market
    /// [R] Rent sysvar
    /// [R] Sytem program
    SetRegistryConfig {
        /// Set registry config params
        params: SetRegistryConfigParams,
        /// Set pool markets configuration
        pool_markets_cfg: PoolMarketsConfig,
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
    general_pool_market: &Pubkey,
    income_pool_market: &Pubkey,
    ulp_pool_market: &Pubkey,
    params: SetRegistryConfigParams,
    pool_markets_cfg: PoolMarketsConfig,
) -> Instruction {
    let (registry_config, _) = find_config_program_address(program_id, registry);

    let accounts = vec![
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new(registry_config, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(*general_pool_market, false),
        AccountMeta::new_readonly(*income_pool_market, false),
        AccountMeta::new_readonly(*ulp_pool_market, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &RegistryInstruction::SetRegistryConfig {
            params,
            pool_markets_cfg,
        },
        accounts,
    )
}
