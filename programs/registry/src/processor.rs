//! Program state processor

use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use everlend_utils::{
    assert_account_key, assert_owned_by, assert_rent_exempt, assert_signer, assert_uninitialized,
    cpi, EverlendError,
};

use crate::state::DeprecatedRegistryConfig;
use crate::{
    find_config_program_address,
    instruction::RegistryInstruction,
    state::{
        InitRegistryConfigParams, InitRegistryParams, Registry, RegistryConfig, RegistryPrograms,
        RegistryRootAccounts, RegistrySettings,
    },
};

/// Program state handler.
pub struct Processor {}

impl Processor {
    /// Process Init instruction
    pub fn init(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let registry_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;

        assert_rent_exempt(rent, registry_info)?;

        // Check programs
        assert_owned_by(registry_info, program_id)?;

        // Get registry state
        let mut registry = Registry::unpack_unchecked(&registry_info.data.borrow())?;
        assert_uninitialized(&registry)?;

        registry.init(InitRegistryParams {
            manager: *manager_info.key,
        });

        Registry::pack(registry, *registry_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process SetRegistryConfig instruction
    pub fn set_registry_config(
        program_id: &Pubkey,
        programs: RegistryPrograms,
        roots: RegistryRootAccounts,
        settings: RegistrySettings,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let registry_info = next_account_info(account_info_iter)?;
        let registry_config_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        // Check programs
        assert_owned_by(registry_info, program_id)?;

        // Get registry state
        let registry = Registry::unpack(&registry_info.data.borrow())?;
        assert_account_key(manager_info, &registry.manager)?;

        // Check registry config
        let (registry_config_pubkey, bump_seed) =
            find_config_program_address(program_id, registry_info.key);
        assert_account_key(registry_config_info, &registry_config_pubkey)?;

        // Create or get registry config account
        let registry_config = match registry_config_info.lamports() {
            // Create registry config account
            0 => {
                let signers_seeds = &[
                    "config".as_bytes(),
                    &registry_info.key.to_bytes()[..32],
                    &[bump_seed],
                ];

                cpi::system::create_account::<RegistryConfig>(
                    program_id,
                    manager_info.clone(),
                    registry_config_info.clone(),
                    &[signers_seeds],
                    rent,
                )?;

                let mut registry_config =
                    RegistryConfig::unpack_unchecked(&registry_config_info.data.borrow())?;
                registry_config.init(InitRegistryConfigParams {
                    registry: *registry_info.key,
                });

                registry_config
            }
            _ => {
                assert_owned_by(registry_config_info, program_id)?;

                let registry_config = RegistryConfig::unpack(&registry_config_info.data.borrow())?;

                // Check registry config accounts
                assert_account_key(registry_info, &registry_config.registry)?;

                registry_config
            }
        };

        RegistryPrograms::pack_into_slice(&programs, *registry_config_info.data.borrow_mut());
        RegistryRootAccounts::pack_into_slice(&roots, *registry_config_info.data.borrow_mut());
        RegistrySettings::pack_into_slice(&settings, *registry_config_info.data.borrow_mut());

        RegistryConfig::pack(registry_config, *registry_config_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process SetRegistryRootAccounts instruction
    pub fn set_registry_root_accounts(
        program_id: &Pubkey,
        roots: RegistryRootAccounts,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let registry_info = next_account_info(account_info_iter)?;
        let registry_config_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let _system_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        // Check programs
        assert_owned_by(registry_info, program_id)?;
        assert_owned_by(registry_config_info, program_id)?;

        // Get registry state
        let registry = Registry::unpack(&registry_info.data.borrow())?;

        // Check manager
        assert_account_key(manager_info, &registry.manager)?;

        // Check registry config
        let (registry_config_pubkey, _) =
            find_config_program_address(program_id, registry_info.key);
        assert_account_key(registry_config_info, &registry_config_pubkey)?;

        let registry_config = RegistryConfig::unpack(&registry_config_info.data.borrow())?;

        // Check registry config accounts
        assert_account_key(registry_info, &registry_config.registry)?;

        RegistryRootAccounts::pack_into_slice(&roots, *registry_config_info.data.borrow_mut());

        Ok(())
    }

    /// Process CloseRegistryConfig instruction
    pub fn close_registry_config(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let registry_info = next_account_info(account_info_iter)?;
        let registry_config_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        // Check programs
        assert_owned_by(registry_info, program_id)?;
        assert_owned_by(registry_config_info, program_id)?;

        // Get registry state
        let registry = Registry::unpack(&registry_info.data.borrow())?;

        // Check manager
        assert_account_key(manager_info, &registry.manager)?;

        // Check registry config
        let (registry_config_pubkey, _) =
            find_config_program_address(program_id, registry_info.key);
        assert_account_key(registry_config_info, &registry_config_pubkey)?;

        let registry_config =
            DeprecatedRegistryConfig::unpack(&registry_config_info.data.borrow())?;

        // Check registry config accounts
        assert_account_key(registry_info, &registry_config.registry)?;

        // Close registry config account and return rent
        let from_starting_lamports = manager_info.lamports();
        let deprecated_account_lamports = registry_config_info.lamports();

        **registry_config_info.lamports.borrow_mut() = 0;
        **manager_info.lamports.borrow_mut() = from_starting_lamports
            .checked_add(deprecated_account_lamports)
            .ok_or(EverlendError::MathOverflow)?;

        Ok(())
    }

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
                Self::init(program_id, accounts)
            }

            RegistryInstruction::SetRegistryConfig {
                programs,
                roots,
                settings,
            } => {
                msg!("RegistryInstruction: SetRegistryConfig");
                Self::set_registry_config(program_id, programs, roots, settings, accounts)
            }

            RegistryInstruction::SetRegistryRootAccounts { roots } => {
                msg!("RegistryInstruction: SetRegistryRootAccounts");
                Self::set_registry_root_accounts(program_id, roots, accounts)
            }

            RegistryInstruction::CloseRegistryConfig => {
                msg!("RegistryInstruction: CloseRegistryConfig");
                Self::close_registry_config(program_id, accounts)
            }
        }
    }
}
