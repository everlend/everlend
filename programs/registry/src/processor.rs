//! Program state processor

use std::convert::TryInto;

use borsh::BorshDeserialize;
use solana_program::program_error::ProgramError;
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

use crate::deprecated::deprecated_find_config_program_address;
use crate::state::{DeprecatedRegistryConfig, PoolMarketsConfig, TOTAL_DISTRIBUTIONS};
use crate::{
    find_config_program_address,
    instruction::RegistryInstruction,
    seed,
    state::{
        InitRegistryConfigParams, InitRegistryParams, Registry, RegistryConfig,
        SetRegistryConfigParams,
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
        params: SetRegistryConfigParams,
        pool_markets_cfg: PoolMarketsConfig,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let registry_info = next_account_info(account_info_iter)?;
        let registry_config_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let general_pool_market_info = next_account_info(account_info_iter)?;
        let income_pool_market_info = next_account_info(account_info_iter)?;
        let ulp_pool_markets_info = account_info_iter;

        assert_signer(manager_info)?;
        assert_owned_by(registry_info, program_id)?;

        // Get registry state
        let registry = Registry::unpack(&registry_info.data.borrow())?;
        assert_account_key(manager_info, &registry.manager)?;
        assert_account_key(
            general_pool_market_info,
            &pool_markets_cfg.general_pool_market,
        )?;
        assert_account_key(
            income_pool_market_info,
            &pool_markets_cfg.income_pool_market,
        )?;

        let general_pool_market = everlend_general_pool::state::PoolMarket::unpack(
            &general_pool_market_info.data.borrow(),
        )?;
        assert_account_key(manager_info, &general_pool_market.manager)?;

        let income_pool_market = everlend_income_pools::state::IncomePoolMarket::unpack(
            &income_pool_market_info.data.borrow(),
        )?;
        assert_account_key(manager_info, &income_pool_market.manager)?;
        assert_account_key(
            general_pool_market_info,
            &income_pool_market.general_pool_market,
        )?;

        pool_markets_cfg
            .iter_filtered_ulp_pool_markets()
            .map(|ulp_pool_market_pubkey| {
                next_account_info(ulp_pool_markets_info).and_then(|ulp_pool_market_info| {
                    assert_account_key(ulp_pool_market_info, ulp_pool_market_pubkey).and_then(
                        |_| {
                            everlend_ulp::state::PoolMarket::unpack(
                                &ulp_pool_market_info.data.borrow(),
                            )
                            .and_then(|ulp_pool_market| {
                                assert_account_key(manager_info, &ulp_pool_market.manager)
                            })
                        },
                    )
                })
            })
            .collect::<Result<(), ProgramError>>()?;

        let (registry_config_pubkey, bump_seed) =
            find_config_program_address(program_id, registry_info.key);
        assert_account_key(registry_config_info, &registry_config_pubkey)?;

        msg!("1");

        // Create or get registry config account
        let mut registry_config = match registry_config_info.lamports() {
            // Create registry config account
            0 => {
                let config_program_address_seed = seed();

                let signers_seeds = &[
                    config_program_address_seed.as_bytes(),
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
                let registry_config = RegistryConfig::unpack(&registry_config_info.data.borrow())?;
                assert_account_key(registry_info, &registry_config.registry)?;

                registry_config
            }
        };

        msg!("2");

        assert_owned_by(registry_config_info, program_id)?;

        // Set registry config
        registry_config.set(params, pool_markets_cfg);

        RegistryConfig::pack(registry_config, *registry_config_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process MigrateRegistryConfig instruction
    pub fn migrate_registry_config(
        program_id: &Pubkey,
        pool_markets_cfg: PoolMarketsConfig,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let registry_info = next_account_info(account_info_iter)?;
        let deprecated_registry_config_info = next_account_info(account_info_iter)?;
        let registry_config_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let general_pool_market_info = next_account_info(account_info_iter)?;
        let income_pool_market_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let ulp_pool_markets_info = account_info_iter;

        assert_signer(manager_info)?;
        assert_owned_by(registry_info, program_id)?;
        assert_owned_by(deprecated_registry_config_info, program_id)?;

        let (registry_config_pubkey, bump_seed) =
            find_config_program_address(program_id, registry_info.key);
        assert_account_key(registry_config_info, &registry_config_pubkey)?;

        let (deprecated_registry_config_pubkey, _) =
            deprecated_find_config_program_address(program_id, registry_info.key);
        assert_account_key(
            deprecated_registry_config_info,
            &deprecated_registry_config_pubkey,
        )?;

        // Get registry state
        let registry = Registry::unpack(&registry_info.data.borrow())?;
        assert_account_key(manager_info, &registry.manager)?;
        assert_account_key(
            general_pool_market_info,
            &pool_markets_cfg.general_pool_market,
        )?;
        assert_account_key(
            income_pool_market_info,
            &pool_markets_cfg.income_pool_market,
        )?;

        let general_pool_market = everlend_general_pool::state::PoolMarket::unpack(
            &general_pool_market_info.data.borrow(),
        )?;
        assert_account_key(manager_info, &general_pool_market.manager)?;

        let income_pool_market = everlend_income_pools::state::IncomePoolMarket::unpack(
            &income_pool_market_info.data.borrow(),
        )?;
        assert_account_key(manager_info, &income_pool_market.manager)?;
        assert_account_key(
            general_pool_market_info,
            &income_pool_market.general_pool_market,
        )?;

        pool_markets_cfg
            .iter_filtered_ulp_pool_markets()
            .map(|ulp_pool_market_pubkey| {
                next_account_info(ulp_pool_markets_info).and_then(|ulp_pool_market_info| {
                    assert_account_key(ulp_pool_market_info, ulp_pool_market_pubkey).and_then(
                        |_| {
                            everlend_ulp::state::PoolMarket::unpack(
                                &ulp_pool_market_info.data.borrow(),
                            )
                            .and_then(|ulp_pool_market| {
                                assert_account_key(manager_info, &ulp_pool_market.manager)
                            })
                        },
                    )
                })
            })
            .collect::<Result<(), ProgramError>>()?;

        let deprecated_registry_config =
            DeprecatedRegistryConfig::unpack(&deprecated_registry_config_info.data.borrow())?;
        assert_account_key(registry_info, &deprecated_registry_config.registry)?;

        let from_starting_lamports = manager_info.lamports();
        let deprecated_lamports = deprecated_registry_config_info.lamports();

        **deprecated_registry_config_info.lamports.borrow_mut() = 0;
        **manager_info.lamports.borrow_mut() = from_starting_lamports
            .checked_add(deprecated_lamports)
            .ok_or(EverlendError::MathOverflow)?;

        let config_program_address_seed = seed();

        let signers_seeds = &[
            config_program_address_seed.as_bytes(),
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

        let registry_actual =
            RegistryConfig::unpack_unchecked(*registry_config_info.data.borrow())?;
        assert_uninitialized(&registry_actual)?;

        let params = SetRegistryConfigParams {
            general_pool_program_id: deprecated_registry_config.general_pool_program_id,
            ulp_program_id: deprecated_registry_config.ulp_program_id,
            liquidity_oracle_program_id: deprecated_registry_config.liquidity_oracle_program_id,
            depositor_program_id: deprecated_registry_config.depositor_program_id,
            income_pools_program_id: deprecated_registry_config.income_pools_program_id,
            money_market_program_ids: deprecated_registry_config.money_market_program_ids
                [..TOTAL_DISTRIBUTIONS]
                .try_into()
                .unwrap(),
            refresh_income_interval: deprecated_registry_config.refresh_income_interval,
        };
        // Set registry config
        let mut registry_config = RegistryConfig::default();
        registry_config.set(params, pool_markets_cfg);

        RegistryConfig::pack(registry_config, *registry_config_info.data.borrow_mut())?;

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
                params,
                pool_markets_cfg,
            } => {
                msg!("RegistryInstruction: SetRegistryConfig");
                Self::set_registry_config(program_id, params, pool_markets_cfg, accounts)
            }

            RegistryInstruction::MigrateRegistryConfig { pool_markets_cfg } => {
                msg!("RegistryInstruction: MigrateRegistryConfig");
                Self::migrate_registry_config(program_id, pool_markets_cfg, accounts)
            }
        }
    }
}
