//! Program state processor

use std::cmp::Ordering;

use borsh::BorshDeserialize;
use solana_program::program_error::ProgramError;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_token::state::Account;

use everlend_general_pool::{
    find_withdrawal_requests_program_address,
    state::{Pool, WithdrawalRequests},
};
use everlend_liquidity_oracle::{
    find_liquidity_oracle_token_distribution_program_address, state::TokenDistribution,
};
use everlend_registry::{
    find_config_program_address,
    state::{RegistryPrograms, RegistryRootAccounts, RegistrySettings},
};
use everlend_utils::{
    assert_account_key, assert_owned_by, assert_rent_exempt, assert_uninitialized, cpi,
    find_program_address, EverlendError,
};

use crate::{
    find_rebalancing_program_address, find_transit_program_address,
    instruction::DepositorInstruction,
    state::{
        Depositor, InitDepositorParams, InitRebalancingParams, Rebalancing, RebalancingOperation,
    },
    utils::{deposit, withdraw},
};

/// Program state handler.
pub struct Processor {}

impl Processor {
    /// Process Init instruction
    pub fn init(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let depositor_info = next_account_info(account_info_iter)?;
        let registry_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;

        assert_rent_exempt(rent, depositor_info)?;

        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(registry_info, &everlend_registry::id())?;

        // Get depositor state
        let mut depositor = Depositor::unpack_unchecked(&depositor_info.data.borrow())?;
        assert_uninitialized(&depositor)?;

        depositor.init(InitDepositorParams {
            registry: *registry_info.key,
        });

        Depositor::pack(depositor, *depositor_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process CreateTransit instruction
    pub fn create_transit(
        program_id: &Pubkey,
        seed: String,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let depositor_info = next_account_info(account_info_iter)?;
        let transit_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;
        let from_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(depositor_info, program_id)?;

        // Get depositor state
        // Check initialized
        Depositor::unpack(&depositor_info.data.borrow())?;

        // Create transit account for SPL program
        let (transit_pubkey, bump_seed) =
            find_transit_program_address(program_id, depositor_info.key, mint_info.key, &seed);
        assert_account_key(transit_info, &transit_pubkey)?;

        let signers_seeds = &[
            seed.as_bytes(),
            &depositor_info.key.to_bytes()[..32],
            &mint_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        cpi::system::create_account::<spl_token::state::Account>(
            &spl_token::id(),
            from_info.clone(),
            transit_info.clone(),
            &[signers_seeds],
            rent,
        )?;

        // Initialize transit token account for spl token
        cpi::spl_token::initialize_account(
            transit_info.clone(),
            mint_info.clone(),
            depositor_authority_info.clone(),
            rent_info.clone(),
        )?;

        Ok(())
    }

    /// Process StartRebalancing instruction
    pub fn start_rebalancing(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        refresh_income: bool,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let registry_config_info = next_account_info(account_info_iter)?;

        let depositor_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;
        let rebalancing_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;

        let general_pool_market_info = next_account_info(account_info_iter)?;
        let general_pool_market_authority_info = next_account_info(account_info_iter)?;
        let general_pool_info = next_account_info(account_info_iter)?;
        let general_pool_token_account_info = next_account_info(account_info_iter)?;
        let general_pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let withdrawal_requests_info = next_account_info(account_info_iter)?;

        let liquidity_transit_info = next_account_info(account_info_iter)?;

        // TODO: we can do it optional for refresh income case in the future
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let token_distribution_info = next_account_info(account_info_iter)?;
        let from_info = next_account_info(account_info_iter)?;

        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;
        let _liquidity_oracle_program_info = next_account_info(account_info_iter)?;
        let _general_pool_program_info = next_account_info(account_info_iter)?;

        // Get depositor state
        let depositor = Depositor::unpack(&depositor_info.data.borrow())?;

        // Check registry
        let (registry_config_pubkey, _) =
            find_config_program_address(&everlend_registry::id(), &depositor.registry);
        assert_account_key(registry_config_info, &registry_config_pubkey)?;

        // TODO: We can check registry config as well for initialized
        let programs = RegistryPrograms::unpack_from_slice(&registry_config_info.data.borrow())?;
        let roots = RegistryRootAccounts::unpack_from_slice(&registry_config_info.data.borrow())?;
        let settings = RegistrySettings::unpack_from_slice(&registry_config_info.data.borrow())?;

        // Check programs
        assert_owned_by(registry_config_info, &everlend_registry::id())?;
        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(
            token_distribution_info,
            &programs.liquidity_oracle_program_id,
        )?;
        assert_owned_by(general_pool_info, &programs.general_pool_program_id)?;
        assert_owned_by(withdrawal_requests_info, &programs.general_pool_program_id)?;

        // Check root accounts
        assert_account_key(general_pool_market_info, &roots.general_pool_market)?;
        assert_account_key(liquidity_oracle_info, &roots.liquidity_oracle)?;

        // Check rebalancing
        let (rebalancing_pubkey, bump_seed) =
            find_rebalancing_program_address(program_id, depositor_info.key, mint_info.key);
        assert_account_key(rebalancing_info, &rebalancing_pubkey)?;

        // Create or get rebalancing account
        let mut rebalancing = match rebalancing_info.lamports() {
            // Create rebalancing account
            0 => {
                let signers_seeds = &[
                    "rebalancing".as_bytes(),
                    &depositor_info.key.to_bytes()[..32],
                    &mint_info.key.to_bytes()[..32],
                    &[bump_seed],
                ];

                cpi::system::create_account::<Rebalancing>(
                    program_id,
                    from_info.clone(),
                    rebalancing_info.clone(),
                    &[signers_seeds],
                    rent,
                )?;

                let mut rebalancing =
                    Rebalancing::unpack_unchecked(&rebalancing_info.data.borrow())?;
                rebalancing.init(InitRebalancingParams {
                    depositor: *depositor_info.key,
                    mint: *mint_info.key,
                });

                rebalancing
            }
            _ => {
                let rebalancing = Rebalancing::unpack(&rebalancing_info.data.borrow())?;
                assert_account_key(depositor_info, &rebalancing.depositor)?;
                assert_account_key(mint_info, &rebalancing.mint)?;

                rebalancing
            }
        };

        assert_owned_by(rebalancing_info, program_id)?;

        // Check rebalancing is completed
        if !rebalancing.is_completed() {
            return Err(EverlendError::IncompleteRebalancing.into());
        }

        // Check token distribution
        let (token_distribution_pubkey, _) =
            find_liquidity_oracle_token_distribution_program_address(
                &programs.liquidity_oracle_program_id,
                liquidity_oracle_info.key,
                mint_info.key,
            );
        assert_account_key(token_distribution_info, &token_distribution_pubkey)?;
        let new_token_distribution =
            TokenDistribution::unpack(&token_distribution_info.data.borrow())?;

        // Check general pool
        let (general_pool_pubkey, _) = everlend_general_pool::find_pool_program_address(
            &programs.general_pool_program_id,
            general_pool_market_info.key,
            mint_info.key,
        );
        assert_account_key(general_pool_info, &general_pool_pubkey)?;

        let general_pool =
            everlend_general_pool::state::Pool::unpack(&general_pool_info.data.borrow())?;

        // Check general pool accounts
        assert_account_key(general_pool_market_info, &general_pool.pool_market)?;
        assert_account_key(general_pool_token_account_info, &general_pool.token_account)?;
        assert_account_key(mint_info, &general_pool.token_mint)?;

        // Check withtdrawal requests
        let (withdrawal_requests_pubkey, _) = find_withdrawal_requests_program_address(
            &programs.general_pool_program_id,
            general_pool_market_info.key,
            &general_pool.token_mint,
        );
        assert_account_key(withdrawal_requests_info, &withdrawal_requests_pubkey)?;
        let withdrawal_requests =
            WithdrawalRequests::unpack(&withdrawal_requests_info.data.borrow())?;

        // Check transit: liquidity
        let (liquidity_transit_pubkey, _) =
            find_transit_program_address(program_id, depositor_info.key, mint_info.key, "");
        assert_account_key(liquidity_transit_info, &liquidity_transit_pubkey)?;

        // Calculate total liquidity supply
        let general_pool_token_account =
            Account::unpack_unchecked(&general_pool_token_account_info.data.borrow())?;

        let liquidity_transit_supply = Account::unpack(&liquidity_transit_info.data.borrow())?
            .amount
            .saturating_sub(rebalancing.unused_liquidity()?);
        msg!("liquidity_transit_supply: {}", liquidity_transit_supply);

        let release_withdrawal_requests_amount = withdrawal_requests
            .liquidity_supply
            .saturating_sub(liquidity_transit_supply);

        let new_distributed_liquidity = general_pool_token_account
            .amount
            .checked_add(rebalancing.distributed_liquidity)
            .ok_or(EverlendError::MathOverflow)?
            .checked_sub(release_withdrawal_requests_amount)
            .ok_or(EverlendError::MathOverflow)?;
        msg!("new_distributed_liquidity: {}", new_distributed_liquidity);

        let borrow_amount =
            new_distributed_liquidity.saturating_sub(rebalancing.distributed_liquidity);
        let amount = (borrow_amount as i64)
            .checked_sub(liquidity_transit_supply as i64)
            .ok_or(EverlendError::MathOverflow)?;
        msg!("amount: {}", amount);

        let (depositor_authority_pubkey, bump_seed) =
            find_program_address(program_id, depositor_info.key);
        assert_account_key(depositor_authority_info, &depositor_authority_pubkey)?;
        let signers_seeds = &[&depositor_info.key.to_bytes()[..32], &[bump_seed]];

        match amount.cmp(&0) {
            Ordering::Greater => {
                msg!("Borrow from General Pool");
                everlend_general_pool::cpi::borrow(
                    general_pool_market_info.clone(),
                    general_pool_market_authority_info.clone(),
                    general_pool_info.clone(),
                    general_pool_borrow_authority_info.clone(),
                    liquidity_transit_info.clone(),
                    general_pool_token_account_info.clone(),
                    depositor_authority_info.clone(),
                    amount.checked_abs().ok_or(EverlendError::MathOverflow)? as u64,
                    &[signers_seeds],
                )?;
            }
            Ordering::Less => {
                msg!("Repay to General Pool");
                everlend_general_pool::cpi::repay(
                    general_pool_market_info.clone(),
                    general_pool_market_authority_info.clone(),
                    general_pool_info.clone(),
                    general_pool_borrow_authority_info.clone(),
                    liquidity_transit_info.clone(),
                    general_pool_token_account_info.clone(),
                    depositor_authority_info.clone(),
                    amount.checked_abs().ok_or(EverlendError::MathOverflow)? as u64,
                    0,
                    &[signers_seeds],
                )?;
            }
            Ordering::Equal => {}
        }

        // Compute rebalancing steps
        msg!("Computing");
        if refresh_income {
            rebalancing.compute_with_refresh_income(
                &programs.money_market_program_ids,
                &settings,
                clock.slot,
                new_distributed_liquidity,
            )?;
        } else {
            rebalancing.compute(
                &programs.money_market_program_ids,
                new_token_distribution,
                new_distributed_liquidity,
            )?;
        }

        // msg!("Steps = {:?}", rebalancing.steps);

        Rebalancing::pack(rebalancing, *rebalancing_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process Deposit instruction
    pub fn deposit(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let registry_config_info = next_account_info(account_info_iter)?;

        let depositor_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;
        let rebalancing_info = next_account_info(account_info_iter)?;

        let collateral_pool_market_info = next_account_info(account_info_iter)?;
        let collateral_pool_market_authority_info = next_account_info(account_info_iter)?;
        let collateral_pool_info = next_account_info(account_info_iter)?;
        let collateral_pool_token_account_info = next_account_info(account_info_iter)?;
        let collateral_pool_collateral_transit_info = next_account_info(account_info_iter)?;
        let collateral_pool_collateral_mint_info = next_account_info(account_info_iter)?;

        let liquidity_transit_info = next_account_info(account_info_iter)?;
        let liquidity_mint_info = next_account_info(account_info_iter)?;
        let collateral_transit_info = next_account_info(account_info_iter)?;
        let collateral_mint_info = next_account_info(account_info_iter)?;

        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;
        let _token_program_info = next_account_info(account_info_iter)?;
        let _everlend_ulp_info = next_account_info(account_info_iter)?;

        let money_market_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(registry_config_info, &everlend_registry::id())?;
        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(rebalancing_info, program_id)?;

        let depositor = Depositor::unpack(&depositor_info.data.borrow())?;

        // Check registry config
        let (registry_config_pubkey, _) =
            find_config_program_address(&everlend_registry::id(), &depositor.registry);
        assert_account_key(registry_config_info, &registry_config_pubkey)?;

        let programs = RegistryPrograms::unpack_from_slice(&registry_config_info.data.borrow())?;
        let root_accounts =
            RegistryRootAccounts::unpack_from_slice(&registry_config_info.data.borrow())?;

        assert_owned_by(collateral_pool_market_info, &programs.ulp_program_id)?;

        // Check collateral pool market
        if !root_accounts
            .collateral_pool_markets
            .contains(collateral_pool_market_info.key)
        {
            return Err(ProgramError::InvalidArgument);
        }

        // Check collateral pool
        let (collateral_pool_pubkey, _) = everlend_ulp::find_pool_program_address(
            &programs.ulp_program_id,
            collateral_pool_market_info.key,
            collateral_mint_info.key,
        );
        assert_account_key(collateral_pool_info, &collateral_pool_pubkey)?;

        let collateral_pool =
            everlend_ulp::state::Pool::unpack(&collateral_pool_info.data.borrow())?;

        // Check collateral pool accounts
        assert_account_key(collateral_mint_info, &collateral_pool.token_mint)?;
        assert_account_key(
            collateral_pool_token_account_info,
            &collateral_pool.token_account,
        )?;
        assert_account_key(
            collateral_pool_collateral_mint_info,
            &collateral_pool.pool_mint,
        )?;

        // Check rebalancing
        let (rebalancing_pubkey, _) = find_rebalancing_program_address(
            program_id,
            depositor_info.key,
            liquidity_mint_info.key,
        );
        assert_account_key(rebalancing_info, &rebalancing_pubkey)?;

        let mut rebalancing = Rebalancing::unpack(&rebalancing_info.data.borrow())?;
        assert_account_key(depositor_info, &rebalancing.depositor)?;
        assert_account_key(liquidity_mint_info, &rebalancing.mint)?;

        if rebalancing.is_completed() {
            return Err(EverlendError::RebalancingIsCompleted.into());
        }

        // Check transit: liquidity
        let (liquidity_transit_pubkey, _) = find_transit_program_address(
            program_id,
            depositor_info.key,
            liquidity_mint_info.key,
            "",
        );
        assert_account_key(liquidity_transit_info, &liquidity_transit_pubkey)?;

        // Check transit: collateral
        let (collateral_transit_pubkey, _) = find_transit_program_address(
            program_id,
            depositor_info.key,
            collateral_mint_info.key,
            "",
        );
        assert_account_key(collateral_transit_info, &collateral_transit_pubkey)?;

        // Create depositor authority account
        let (depositor_authority_pubkey, bump_seed) =
            find_program_address(program_id, depositor_info.key);
        assert_account_key(depositor_authority_info, &depositor_authority_pubkey)?;
        let signers_seeds = &[&depositor_info.key.to_bytes()[..32], &[bump_seed]];

        let step = rebalancing.next_step();

        if programs.money_market_program_ids[usize::from(step.money_market_index)]
            != *money_market_program_info.key
        {
            return Err(EverlendError::InvalidRebalancingMoneyMarket.into());
        }

        msg!("Deposit");
        let collateral_amount = deposit(
            &programs,
            collateral_pool_market_info.clone(),
            collateral_pool_market_authority_info.clone(),
            collateral_pool_info.clone(),
            collateral_pool_token_account_info.clone(),
            collateral_pool_collateral_transit_info.clone(),
            collateral_pool_collateral_mint_info.clone(),
            collateral_transit_info.clone(),
            collateral_mint_info.clone(),
            liquidity_transit_info.clone(),
            liquidity_mint_info.clone(),
            depositor_authority_info.clone(),
            clock_info.clone(),
            money_market_program_info.clone(),
            account_info_iter,
            step.liquidity_amount,
            &[signers_seeds],
        )?;

        rebalancing.execute_step(
            RebalancingOperation::Deposit,
            Some(collateral_amount),
            clock.slot,
        )?;

        Rebalancing::pack(rebalancing, *rebalancing_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process Withdraw instruction
    pub fn withdraw(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let registry_config_info = next_account_info(account_info_iter)?;

        let depositor_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;
        let rebalancing_info = next_account_info(account_info_iter)?;

        let income_pool_market_info = next_account_info(account_info_iter)?;
        let income_pool_info = next_account_info(account_info_iter)?;
        let income_pool_token_account_info = next_account_info(account_info_iter)?;

        let collateral_pool_market_info = next_account_info(account_info_iter)?;
        let collateral_pool_market_authority_info = next_account_info(account_info_iter)?;
        let collateral_pool_info = next_account_info(account_info_iter)?;
        let collateral_pool_token_account_info = next_account_info(account_info_iter)?;
        let collateral_pool_collateral_transit_info = next_account_info(account_info_iter)?;
        let collateral_pool_collateral_mint_info = next_account_info(account_info_iter)?;

        let collateral_transit_info = next_account_info(account_info_iter)?;
        let collateral_mint_info = next_account_info(account_info_iter)?;
        let liquidity_transit_info = next_account_info(account_info_iter)?;
        let liquidity_reserve_transit_info = next_account_info(account_info_iter)?;
        let liquidity_mint_info = next_account_info(account_info_iter)?;

        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;
        let _token_program_info = next_account_info(account_info_iter)?;
        let _everlend_ulp_info = next_account_info(account_info_iter)?;
        let _everlend_income_pools_info = next_account_info(account_info_iter)?;

        let money_market_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(registry_config_info, &everlend_registry::id())?;
        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(rebalancing_info, program_id)?;

        let depositor = Depositor::unpack(&depositor_info.data.borrow())?;

        // Check registry config
        let (registry_config_pubkey, _) =
            find_config_program_address(&everlend_registry::id(), &depositor.registry);
        assert_account_key(registry_config_info, &registry_config_pubkey)?;

        let programs = RegistryPrograms::unpack_from_slice(&registry_config_info.data.borrow())?;

        let root_accounts =
            RegistryRootAccounts::unpack_from_slice(&registry_config_info.data.borrow())?;

        assert_owned_by(collateral_pool_market_info, &programs.ulp_program_id)?;

        // Check collateral pool market
        if !root_accounts
            .collateral_pool_markets
            .contains(collateral_pool_market_info.key)
        {
            return Err(ProgramError::InvalidArgument);
        }

        // Check collateral pool
        let (collateral_pool_pubkey, _) = everlend_ulp::find_pool_program_address(
            &programs.ulp_program_id,
            collateral_pool_market_info.key,
            collateral_mint_info.key,
        );
        assert_account_key(collateral_pool_info, &collateral_pool_pubkey)?;

        let collateral_pool =
            everlend_ulp::state::Pool::unpack(&collateral_pool_info.data.borrow())?;

        // Check collateral pool accounts
        assert_account_key(collateral_mint_info, &collateral_pool.token_mint)?;
        assert_account_key(
            collateral_pool_token_account_info,
            &collateral_pool.token_account,
        )?;
        assert_account_key(
            collateral_pool_collateral_mint_info,
            &collateral_pool.pool_mint,
        )?;

        // Check rebalancing
        let (rebalancing_pubkey, _) = find_rebalancing_program_address(
            program_id,
            depositor_info.key,
            liquidity_mint_info.key,
        );
        assert_account_key(rebalancing_info, &rebalancing_pubkey)?;

        let mut rebalancing = Rebalancing::unpack(&rebalancing_info.data.borrow())?;
        assert_account_key(depositor_info, &rebalancing.depositor)?;
        assert_account_key(liquidity_mint_info, &rebalancing.mint)?;

        if rebalancing.is_completed() {
            return Err(EverlendError::RebalancingIsCompleted.into());
        }

        // Check transit: liquidity
        let (liquidity_transit_pubkey, _) = find_transit_program_address(
            program_id,
            depositor_info.key,
            liquidity_mint_info.key,
            "",
        );
        assert_account_key(liquidity_transit_info, &liquidity_transit_pubkey)?;

        // Check transit: liquidity reserve
        let (liquidity_reserve_transit_pubkey, _) = find_transit_program_address(
            program_id,
            depositor_info.key,
            liquidity_mint_info.key,
            "reserve",
        );
        assert_account_key(
            liquidity_reserve_transit_info,
            &liquidity_reserve_transit_pubkey,
        )?;

        // Check transit: collateral
        let (collateral_transit_pubkey, _) = find_transit_program_address(
            program_id,
            depositor_info.key,
            collateral_mint_info.key,
            "",
        );
        assert_account_key(collateral_transit_info, &collateral_transit_pubkey)?;

        // Create depositor authority account
        let (depositor_authority_pubkey, bump_seed) =
            find_program_address(program_id, depositor_info.key);
        assert_account_key(depositor_authority_info, &depositor_authority_pubkey)?;
        let signers_seeds = &[&depositor_info.key.to_bytes()[..32], &[bump_seed]];

        let step = rebalancing.next_step();

        if programs.money_market_program_ids[usize::from(step.money_market_index)]
            != *money_market_program_info.key
        {
            return Err(EverlendError::InvalidRebalancingMoneyMarket.into());
        }

        msg!("Withdraw");
        withdraw(
            &programs,
            income_pool_market_info.clone(),
            income_pool_info.clone(),
            income_pool_token_account_info.clone(),
            collateral_pool_market_info.clone(),
            collateral_pool_market_authority_info.clone(),
            collateral_pool_info.clone(),
            collateral_pool_token_account_info.clone(),
            collateral_pool_collateral_transit_info.clone(),
            collateral_pool_collateral_mint_info.clone(),
            collateral_transit_info.clone(),
            collateral_mint_info.clone(),
            liquidity_transit_info.clone(),
            liquidity_reserve_transit_info.clone(),
            liquidity_mint_info.clone(),
            depositor_authority_info.clone(),
            clock_info.clone(),
            money_market_program_info.clone(),
            account_info_iter,
            step.collateral_amount.unwrap(),
            step.liquidity_amount,
            &[signers_seeds],
        )?;

        rebalancing.execute_step(RebalancingOperation::Withdraw, None, clock.slot)?;

        Rebalancing::pack(rebalancing, *rebalancing_info.data.borrow_mut())?;

        Ok(())
    }

    /// Instruction processing router
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = DepositorInstruction::try_from_slice(input)?;

        match instruction {
            DepositorInstruction::Init => {
                msg!("DepositorInstruction: Init");
                Self::init(program_id, accounts)
            }

            DepositorInstruction::CreateTransit { seed } => {
                msg!("DepositorInstruction: CreateTransit");
                Self::create_transit(program_id, seed, accounts)
            }

            DepositorInstruction::StartRebalancing { refresh_income } => {
                msg!("DepositorInstruction: StartRebalancing");
                Self::start_rebalancing(program_id, accounts, refresh_income)
            }

            DepositorInstruction::Deposit => {
                msg!("DepositorInstruction: Deposit");
                Self::deposit(program_id, accounts)
            }

            DepositorInstruction::Withdraw => {
                msg!("DepositorInstruction: Withdraw");
                Self::withdraw(program_id, accounts)
            }
        }
    }
}
