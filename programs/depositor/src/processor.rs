//! Program state processor

use borsh::BorshDeserialize;
use everlend_general_pool::{find_withdrawal_requests_program_address, state::WithdrawalRequests};
use everlend_liquidity_oracle::{
    find_token_oracle_program_address,
    state::{DistributionArray, TokenOracle},
};
use everlend_registry::state::{Registry, RegistryMarkets};
use everlend_utils::{
    assert_account_key, assert_owned_by, assert_rent_exempt, assert_signer, assert_uninitialized,
    cpi::{self},
    find_program_address, AccountLoader, EverlendError,
};
use num_traits::Zero;
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
use spl_associated_token_account::get_associated_token_address;
use std::cmp::min;

use crate::instructions::{RefreshMMIncomesContext, WithdrawContext};
use crate::state::{InternalMining, MiningType};
use crate::utils::{deposit, money_market, parse_fill_reward_accounts, FillRewardAccounts};
use crate::{
    find_internal_mining_program_address, find_rebalancing_program_address,
    find_transit_program_address,
    instruction::DepositorInstruction,
    money_market::{CollateralPool, CollateralStorage},
    state::{
        Depositor, InitDepositorParams, InitRebalancingParams, Rebalancing, RebalancingOperation,
    },
};

/// Program state handler.
pub struct Processor {}

impl<'a, 'b> Processor {
    /// Process Init instruction
    pub fn init(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        rebalance_executor: Pubkey,
    ) -> ProgramResult {
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
            rebalance_executor,
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

        // Check programs
        assert_owned_by(depositor_info, program_id)?;

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

        let registry_info = next_account_info(account_info_iter)?;

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
        let token_oracle_info = next_account_info(account_info_iter)?;
        let executor_info = next_account_info(account_info_iter)?;

        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;
        let _liquidity_oracle_program_info = next_account_info(account_info_iter)?;
        let _general_pool_program_info = next_account_info(account_info_iter)?;

        // Check programs
        assert_owned_by(registry_info, &everlend_registry::id())?;
        assert_owned_by(depositor_info, program_id)?;

        // Get depositor state
        let depositor = Depositor::unpack(&depositor_info.data.borrow())?;

        // Check executor
        assert_signer(executor_info)?;
        assert_account_key(executor_info, &depositor.rebalance_executor)?;

        assert_account_key(registry_info, &depositor.registry)?;
        let registry = Registry::unpack(&registry_info.data.borrow())?;

        // Check external programs
        assert_owned_by(token_oracle_info, &everlend_liquidity_oracle::id())?;
        assert_owned_by(general_pool_market_info, &everlend_general_pool::id())?;
        assert_owned_by(general_pool_info, &everlend_general_pool::id())?;
        assert_owned_by(withdrawal_requests_info, &everlend_general_pool::id())?;
        assert_owned_by(liquidity_oracle_info, &everlend_liquidity_oracle::id())?;
        assert_owned_by(token_oracle_info, &everlend_liquidity_oracle::id())?;

        // Check root accounts
        assert_account_key(general_pool_market_info, &registry.general_pool_market)?;
        assert_account_key(liquidity_oracle_info, &registry.liquidity_oracle)?;

        let registry_markets = RegistryMarkets::unpack_from_slice(&registry_info.data.borrow())?;

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
                    executor_info.clone(),
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
                assert_owned_by(rebalancing_info, program_id)?;

                let rebalancing = Rebalancing::unpack(&rebalancing_info.data.borrow())?;
                assert_account_key(depositor_info, &rebalancing.depositor)?;
                assert_account_key(mint_info, &rebalancing.mint)?;

                rebalancing
            }
        };

        // Check rebalancing is completed
        if !rebalancing.is_completed() {
            return Err(EverlendError::IncompleteRebalancing.into());
        }

        {
            // Check token oracle
            let (token_oracle_pubkey, _) = find_token_oracle_program_address(
                &everlend_liquidity_oracle::id(),
                liquidity_oracle_info.key,
                mint_info.key,
            );
            assert_account_key(token_oracle_info, &token_oracle_pubkey)?;

            // Check general pool
            let (general_pool_pubkey, _) = everlend_general_pool::find_pool_program_address(
                &everlend_general_pool::id(),
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

            // Check withdrawal requests
            let (withdrawal_requests_pubkey, _) = find_withdrawal_requests_program_address(
                &everlend_general_pool::id(),
                general_pool_market_info.key,
                &general_pool.token_mint,
            );
            assert_account_key(withdrawal_requests_info, &withdrawal_requests_pubkey)?;

            // Check transit: liquidity
            let (liquidity_transit_pubkey, _) =
                find_transit_program_address(program_id, depositor_info.key, mint_info.key, "");
            assert_account_key(liquidity_transit_info, &liquidity_transit_pubkey)?;
        }

        let general_pool = Account::unpack(&general_pool_token_account_info.data.borrow())?;
        let liquidity_transit = Account::unpack(&liquidity_transit_info.data.borrow())?;
        let withdrawal_requests =
            WithdrawalRequests::unpack(&withdrawal_requests_info.data.borrow())?;

        let available_liquidity = rebalancing
            .distributed_liquidity
            .checked_add(liquidity_transit.amount)
            .ok_or(EverlendError::MathOverflow)?;

        msg!("available_liquidity: {}", available_liquidity);

        // Calculate liquidity to distribute
        let amount_to_distribute = general_pool
            .amount
            .checked_add(available_liquidity)
            .ok_or(EverlendError::MathOverflow)?
            .checked_sub(withdrawal_requests.liquidity_supply)
            .ok_or(EverlendError::MathOverflow)?;

        msg!("amount_to_distribute: {}", amount_to_distribute);

        {
            let (depositor_authority_pubkey, bump_seed) =
                find_program_address(program_id, depositor_info.key);
            assert_account_key(depositor_authority_info, &depositor_authority_pubkey)?;
            let signers_seeds = &[&depositor_info.key.to_bytes()[..32], &[bump_seed]];

            if amount_to_distribute.gt(&available_liquidity) {
                let borrow_amount = amount_to_distribute
                    .checked_sub(available_liquidity)
                    .ok_or(EverlendError::MathOverflow)?;

                msg!("Borrow from General Pool");
                everlend_general_pool::cpi::borrow(
                    general_pool_market_info.clone(),
                    general_pool_market_authority_info.clone(),
                    general_pool_info.clone(),
                    general_pool_borrow_authority_info.clone(),
                    liquidity_transit_info.clone(),
                    general_pool_token_account_info.clone(),
                    depositor_authority_info.clone(),
                    borrow_amount,
                    &[signers_seeds],
                )?;
            } else if !withdrawal_requests.liquidity_supply.is_zero() {
                let repay_amount = withdrawal_requests
                    .liquidity_supply
                    .saturating_sub(general_pool.amount);

                let repay_amount = min(repay_amount, liquidity_transit.amount);
                if !repay_amount.is_zero() {
                    msg!("Repay to General Pool");
                    everlend_general_pool::cpi::repay(
                        general_pool_market_info.clone(),
                        general_pool_market_authority_info.clone(),
                        general_pool_info.clone(),
                        general_pool_borrow_authority_info.clone(),
                        liquidity_transit_info.clone(),
                        general_pool_token_account_info.clone(),
                        depositor_authority_info.clone(),
                        repay_amount,
                        0,
                        &[signers_seeds],
                    )?;
                }
            }
        }

        msg!("Computing");
        if refresh_income {
            rebalancing.compute_with_refresh_income(
                &registry_markets.money_markets,
                registry.refresh_income_interval,
                clock.slot,
                amount_to_distribute,
            )?;
        } else {
            // Compute rebalancing steps
            let token_oracle = TokenOracle::unpack(&token_oracle_info.data.borrow())?;

            rebalancing.compute(
                &registry_markets.money_markets,
                token_oracle,
                amount_to_distribute,
                clock.slot,
            )?;
        }

        Rebalancing::pack(rebalancing, *rebalancing_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process ResetRebalancing instruction
    pub fn set_rebalancing(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount_to_distribute: u64,
        distributed_liquidity: u64,
        distribution_array: DistributionArray,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let registry_info = next_account_info(account_info_iter)?;

        let depositor_info = next_account_info(account_info_iter)?;
        let rebalancing_info = next_account_info(account_info_iter)?;
        let liquidity_mint_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;

        let _system_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        // Check programs
        assert_owned_by(registry_info, &everlend_registry::id())?;
        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(rebalancing_info, program_id)?;

        // Get depositor state
        let depositor = Depositor::unpack(&depositor_info.data.borrow())?;

        // Check registry
        assert_account_key(registry_info, &depositor.registry)?;

        let registry = Registry::unpack(&registry_info.data.borrow())?;

        // Check manager
        assert_account_key(manager_info, &registry.manager)?;

        // Check rebalancing
        let (rebalancing_pubkey, _) = find_rebalancing_program_address(
            program_id,
            depositor_info.key,
            liquidity_mint_info.key,
        );
        assert_account_key(rebalancing_info, &rebalancing_pubkey)?;

        let mut rebalancing = Rebalancing::unpack(&rebalancing_info.data.borrow())?;

        // Check rebalancing accounts
        assert_account_key(depositor_info, &rebalancing.depositor)?;
        assert_account_key(liquidity_mint_info, &rebalancing.mint)?;

        // Check rebalancing is not completed
        if rebalancing.is_completed() {
            return Err(EverlendError::RebalancingIsCompleted.into());
        }

        rebalancing.set(
            amount_to_distribute,
            distributed_liquidity,
            distribution_array,
        )?;

        Rebalancing::pack(rebalancing, *rebalancing_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process Deposit instruction
    pub fn deposit(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter().enumerate();
        // TODO check
        let registry_info = AccountLoader::next_unchecked(account_info_iter)?;

        let depositor_info = AccountLoader::next_unchecked(account_info_iter)?;
        let depositor_authority_info = AccountLoader::next_unchecked(account_info_iter)?;
        let rebalancing_info = AccountLoader::next_unchecked(account_info_iter)?;

        let liquidity_transit_info = AccountLoader::next_unchecked(account_info_iter)?;
        let liquidity_mint_info = AccountLoader::next_unchecked(account_info_iter)?;
        let collateral_transit_info = AccountLoader::next_unchecked(account_info_iter)?;
        let collateral_mint_info = AccountLoader::next_unchecked(account_info_iter)?;

        let executor_info = AccountLoader::next_unchecked(account_info_iter)?;

        let clock_info = AccountLoader::next_unchecked(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;
        let _token_program_info = AccountLoader::next_unchecked(account_info_iter)?;

        let money_market_program_info = AccountLoader::next_unchecked(account_info_iter)?;
        let internal_mining_info = AccountLoader::next_unchecked(account_info_iter)?;

        // Check programs
        assert_owned_by(registry_info, &everlend_registry::id())?;
        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(rebalancing_info, program_id)?;
        let depositor = Depositor::unpack(&depositor_info.data.borrow())?;

        // Check executor
        assert_signer(executor_info)?;
        assert_account_key(executor_info, &depositor.rebalance_executor)?;

        assert_account_key(registry_info, &depositor.registry)?;
        let registry_markets = RegistryMarkets::unpack_from_slice(&registry_info.data.borrow())?;

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

        let (internal_mining_pubkey, _) = find_internal_mining_program_address(
            program_id,
            liquidity_mint_info.key,
            collateral_mint_info.key,
            depositor_info.key,
        );
        assert_account_key(internal_mining_info, &internal_mining_pubkey)?;

        // let money_market =
        let (money_market, is_mining) = money_market(
            &registry_markets,
            program_id,
            money_market_program_info,
            account_info_iter,
            internal_mining_info,
            collateral_mint_info.key,
            depositor_authority_info.key,
        )?;

        let collateral_stor: Option<Box<dyn CollateralStorage>> = {
            if !is_mining {
                let coll_pool = CollateralPool::init(
                    &registry_markets,
                    collateral_mint_info,
                    depositor_authority_info,
                    account_info_iter,
                    false,
                )?;
                Some(Box::new(coll_pool))
            } else {
                None
            }
        };

        {
            let step = rebalancing.next_step();

            if step.operation != RebalancingOperation::Deposit {
                return Err(EverlendError::InvalidRebalancingOperation.into());
            }

            if registry_markets.money_markets[usize::from(step.money_market_index)]
                != *money_market_program_info.key
            {
                return Err(EverlendError::InvalidRebalancingMoneyMarket.into());
            }
            msg!("Deposit");
            let collateral_amount = if step.liquidity_amount.eq(&0) {
                0
            } else {
                deposit(
                    collateral_transit_info,
                    collateral_mint_info,
                    liquidity_transit_info,
                    depositor_authority_info,
                    clock_info,
                    &money_market,
                    is_mining,
                    collateral_stor,
                    step.liquidity_amount,
                    &[signers_seeds],
                )?
            };

            rebalancing.execute_step(
                RebalancingOperation::Deposit,
                Some(collateral_amount),
                clock.slot,
            )?;
        }

        Rebalancing::pack(rebalancing, *rebalancing_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process Withdraw instruction
    // pub fn withdraw(
    //     program_id: &Pubkey,
    //     account_info_iter: &'a mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    // ) -> ProgramResult {
    // }

    /// Process MigrateDepositor instruction
    pub fn migrate_depositor(_program_id: &Pubkey, _accounts: &[AccountInfo]) -> ProgramResult {
        Err(EverlendError::TemporaryUnavailable.into())
    }

    /// Process InitMiningAccount instruction
    pub fn init_mining_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        mining_type: MiningType,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let internal_mining_info = next_account_info(account_info_iter)?;
        let liquidity_mint_info = next_account_info(account_info_iter)?;
        let collateral_mint_info = next_account_info(account_info_iter)?;
        let depositor_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;
        let registry_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;
        assert_owned_by(registry_info, &everlend_registry::id())?;
        assert_owned_by(depositor_info, program_id)?;

        let depositor = Depositor::unpack(&depositor_info.data.borrow())?;
        assert_account_key(registry_info, &depositor.registry)?;

        let registry = Registry::unpack(&registry_info.data.borrow())?;
        assert_account_key(manager_info, &registry.manager)?;

        let (internal_mining_pubkey, internal_mining_bump_seed) =
            find_internal_mining_program_address(
                program_id,
                liquidity_mint_info.key,
                collateral_mint_info.key,
                depositor_info.key,
            );
        assert_account_key(internal_mining_info, &internal_mining_pubkey)?;

        // Check depositor authority account
        let (depositor_authority_pubkey, bump_seed) =
            find_program_address(program_id, depositor_info.key);
        assert_account_key(depositor_authority_info, &depositor_authority_pubkey)?;
        let signers_seeds = &[&depositor_info.key.to_bytes()[..32], &[bump_seed]];

        // Create internal mining account
        if !internal_mining_info.owner.eq(program_id) {
            let signers_seeds = &[
                "internal_mining".as_bytes(),
                &liquidity_mint_info.key.to_bytes()[..32],
                &collateral_mint_info.key.to_bytes()[..32],
                &depositor_info.key.to_bytes()[..32],
                &[internal_mining_bump_seed],
            ];

            cpi::system::create_account::<InternalMining>(
                program_id,
                manager_info.clone(),
                internal_mining_info.clone(),
                &[signers_seeds],
                rent,
            )?;
        } else {
            assert_owned_by(internal_mining_info, program_id)?;
            // Check that account
            InternalMining::unpack(&internal_mining_info.data.borrow())?;
        }

        let staking_program_id_info = next_account_info(account_info_iter)?;

        match mining_type {
            MiningType::Larix {
                mining_account,
                additional_reward_token_account,
            } => {
                let mining_account_info = next_account_info(account_info_iter)?;
                assert_owned_by(mining_account_info, staking_program_id_info.key)?;
                assert_account_key(mining_account_info, &mining_account)?;

                let lending_market_info = next_account_info(account_info_iter)?;
                if let Some(additional_reward_token_account) = additional_reward_token_account {
                    let additional_reward_token_account_info =
                        next_account_info(account_info_iter)?;
                    assert_account_key(
                        additional_reward_token_account_info,
                        &additional_reward_token_account,
                    )?;

                    assert_owned_by(additional_reward_token_account_info, &spl_token::id())?;

                    let token_account = spl_token::state::Account::unpack(
                        &additional_reward_token_account_info.data.borrow(),
                    )?;

                    let (depositor_authority_pubkey, _) =
                        find_program_address(program_id, depositor_info.key);
                    if !token_account.owner.eq(&depositor_authority_pubkey) {
                        return Err(EverlendError::InvalidAccountOwner.into());
                    }
                }

                cpi::larix::init_mining(
                    staking_program_id_info.key,
                    mining_account_info.clone(),
                    depositor_authority_info.clone(),
                    lending_market_info.clone(),
                    &[signers_seeds.as_ref()],
                )?
            }
            MiningType::PortFinance {
                staking_program_id,
                staking_account,
                staking_pool,
                obligation,
            } => {
                assert_account_key(staking_program_id_info, &staking_program_id)?;
                let staking_pool_info = next_account_info(account_info_iter)?;
                let staking_account_info = next_account_info(account_info_iter)?;

                assert_account_key(staking_pool_info, &staking_pool)?;
                assert_account_key(staking_account_info, &staking_account)?;

                if staking_account_info.owner != staking_program_id_info.key {
                    return Err(EverlendError::InvalidAccountOwner.into());
                };

                let money_market_program_id_info = next_account_info(account_info_iter)?;
                let obligation_info = next_account_info(account_info_iter)?;
                assert_account_key(obligation_info, &obligation)?;

                let lending_market_info = next_account_info(account_info_iter)?;
                let clock_info = next_account_info(account_info_iter)?;
                let _spl_token_program = next_account_info(account_info_iter)?;

                cpi::port_finance::init_obligation(
                    money_market_program_id_info.key,
                    obligation_info.clone(),
                    lending_market_info.clone(),
                    depositor_authority_info.clone(),
                    clock_info.clone(),
                    rent_info.clone(),
                    &[signers_seeds.as_ref()],
                )?;

                cpi::port_finance::create_stake_account(
                    staking_program_id_info.key,
                    staking_account_info.clone(),
                    staking_pool_info.clone(),
                    depositor_authority_info.clone(),
                    rent_info.clone(),
                )?;
            }
            MiningType::Quarry {
                rewarder,
            } => {
                assert_account_key(staking_program_id_info, &cpi::quarry::staking_program_id())?;

                let rewarder_info = next_account_info(account_info_iter)?;
                assert_account_key(rewarder_info, &rewarder)?;

                let quarry_info = next_account_info(account_info_iter)?;
                let (quarry, _) = cpi::quarry::find_quarry_program_address(
                    &cpi::quarry::staking_program_id(),
                    &rewarder,
                    collateral_mint_info.key,
                );
                assert_account_key(quarry_info, &quarry)?;

                let miner_info = next_account_info(account_info_iter)?;
                let (miner_pubkey, _) = cpi::quarry::find_miner_program_address(
                    &cpi::quarry::staking_program_id(),
                    &quarry,
                    depositor_authority_info.key,
                );
                assert_account_key(miner_info, &miner_pubkey)?;

                let miner_vault_info = next_account_info(account_info_iter)?;
                let miner_vault = get_associated_token_address(&miner_pubkey, collateral_mint_info.key);
                assert_account_key(miner_vault_info, &miner_vault)?;

                let _spl_token_program = next_account_info(account_info_iter)?;

                cpi::quarry::create_miner(
                    staking_program_id_info.key,
                    depositor_authority_info.clone(),
                    miner_info.clone(),
                    quarry_info.clone(),
                    rewarder_info.clone(),
                    manager_info.clone(),
                    collateral_mint_info.clone(),
                    miner_vault_info.clone(),
                    &[signers_seeds.as_ref()],
                )?;
            }
            MiningType::None => {}
        }

        let mut internal_mining =
            InternalMining::unpack_unchecked(&internal_mining_info.data.borrow())?;
        internal_mining.init(mining_type);

        InternalMining::pack(internal_mining, *internal_mining_info.data.borrow_mut())?;
        Ok(())
    }

    /// Process ClaimMiningReward instruction
    pub fn claim_mining_reward(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        with_subrewards: bool,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let depositor_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;

        let depositor = Depositor::unpack(&depositor_info.data.borrow())?;
        let executor_info = next_account_info(account_info_iter)?;
        // Check executor
        assert_signer(executor_info)?;
        assert_account_key(executor_info, &depositor.rebalance_executor)?;

        let liquidity_mint_info = next_account_info(account_info_iter)?;
        let collateral_mint_info = next_account_info(account_info_iter)?;

        let internal_mining_info = next_account_info(account_info_iter)?;
        let (internal_mining_pubkey, _) = find_internal_mining_program_address(
            program_id,
            liquidity_mint_info.key,
            collateral_mint_info.key,
            depositor_info.key,
        );
        assert_account_key(internal_mining_info, &internal_mining_pubkey)?;
        assert_owned_by(internal_mining_info, program_id)?;

        let internal_mining_type =
            InternalMining::unpack(&internal_mining_info.data.borrow())?.mining_type;

        let token_program_info = next_account_info(account_info_iter)?;
        let staking_program_id_info = next_account_info(account_info_iter)?;
        let eld_reward_program_id = next_account_info(account_info_iter)?;

        // Get reward_pool struct and check liquidity_mint
        let reward_pool_info = next_account_info(account_info_iter)?;
        // TODO fix unpack and check liquidity mint
        // let reward_pool = RewardPool::try_from_slice(&reward_pool_info.data.borrow()[8..])?;
        // assert_account_key(liquidity_mint_info, &reward_pool.liquidity_mint)?;

        let reward_accounts = parse_fill_reward_accounts(
            program_id,
            depositor_info.key,
            reward_pool_info.key,
            eld_reward_program_id.key,
            account_info_iter,
            true,
        )?;

        let mut fill_sub_rewards_accounts: Option<FillRewardAccounts> = None;

        // Create depositor authority account
        let (depositor_authority_pubkey, bump_seed) =
            find_program_address(program_id, depositor_info.key);
        assert_account_key(depositor_authority_info, &depositor_authority_pubkey)?;
        let signers_seeds = &[&depositor_info.key.to_bytes()[..32], &[bump_seed]];

        match internal_mining_type {
            MiningType::Larix {
                mining_account,
                additional_reward_token_account,
            } => {
                if with_subrewards != additional_reward_token_account.is_some() {
                    return Err(ProgramError::InvalidArgument);
                };

                // Parse and check additional reward token account
                if with_subrewards {
                    let sub_reward_accounts = parse_fill_reward_accounts(
                        program_id,
                        depositor_info.key,
                        reward_pool_info.key,
                        eld_reward_program_id.key,
                        account_info_iter,
                        //Larix has manual distribution of subreward
                        false,
                    )?;

                    // Assert additional reward token account
                    assert_account_key(
                        &sub_reward_accounts.reward_transit_info,
                        &additional_reward_token_account.unwrap(),
                    )?;

                    fill_sub_rewards_accounts = Some(sub_reward_accounts);
                };

                let mining_account_info = next_account_info(account_info_iter)?;
                assert_account_key(mining_account_info, &mining_account)?;

                let mine_supply_info = next_account_info(account_info_iter)?;
                let lending_market_info = next_account_info(account_info_iter)?;
                let lending_market_authority_info = next_account_info(account_info_iter)?;
                let reserve_info = next_account_info(account_info_iter)?;
                let reserve_liquidity_oracle = next_account_info(account_info_iter)?;

                cpi::larix::refresh_mine(
                    staking_program_id_info.key,
                    mining_account_info.clone(),
                    reserve_info.clone(),
                )?;

                cpi::larix::refresh_reserve(
                    staking_program_id_info.key,
                    reserve_info.clone(),
                    reserve_liquidity_oracle.clone(),
                )?;
                cpi::larix::claim_mine(
                    staking_program_id_info.key,
                    mining_account_info.clone(),
                    mine_supply_info.clone(),
                    reward_accounts.reward_transit_info.clone(),
                    depositor_authority_info.clone(),
                    lending_market_info.clone(),
                    lending_market_authority_info.clone(),
                    reserve_info.clone(),
                    &[signers_seeds.as_ref()],
                )?;
            }
            MiningType::PortFinance {
                staking_account,
                staking_pool,
                staking_program_id,
                ..
            } => {
                if with_subrewards {
                    let sub_reward_accounts = parse_fill_reward_accounts(
                        program_id,
                        depositor_info.key,
                        reward_pool_info.key,
                        eld_reward_program_id.key,
                        account_info_iter,
                        true,
                    )?;
                    fill_sub_rewards_accounts = Some(sub_reward_accounts.clone());
                }

                let stake_account_info = next_account_info(account_info_iter)?;
                assert_account_key(stake_account_info, &staking_account)?;

                let staking_pool_info = next_account_info(account_info_iter)?;
                assert_account_key(staking_pool_info, &staking_pool)?;

                let staking_pool_authority_info = next_account_info(account_info_iter)?;

                assert_account_key(staking_pool_info, &staking_pool)?;
                assert_account_key(staking_program_id_info, &staking_program_id)?;

                let reward_token_pool = next_account_info(account_info_iter)?;

                let clock = next_account_info(account_info_iter)?;

                // let sub_reward_token_pool_option :Option<AccountInfo>;
                // let sub_reward_destination_option :Option<AccountInfo>;
                let sub_reward = if with_subrewards {
                    let sub_reward_token_pool = next_account_info(account_info_iter)?;

                    // Make local copy
                    let sub_reward_destination = fill_sub_rewards_accounts.unwrap().clone();
                    fill_sub_rewards_accounts = Some(sub_reward_destination.clone());

                    Some((
                        sub_reward_token_pool,
                        sub_reward_destination.reward_transit_info,
                    ))
                } else {
                    None
                };

                cpi::port_finance::claim_reward(
                    staking_program_id_info.key,
                    depositor_authority_info.clone(),
                    stake_account_info.clone(),
                    staking_pool_info.clone(),
                    staking_pool_authority_info.clone(),
                    reward_token_pool.clone(),
                    reward_accounts.reward_transit_info.clone(),
                    sub_reward,
                    clock.clone(),
                    token_program_info.clone(),
                    &[signers_seeds.as_ref()],
                )?;
            }
            MiningType::Quarry {
                rewarder,
            } => {
                assert_account_key(staking_program_id_info,&cpi::quarry::staking_program_id())?;
                let mint_wrapper = next_account_info(account_info_iter)?;
                let mint_wrapper_program = next_account_info(account_info_iter)?;
                let minter = next_account_info(account_info_iter)?;
                // IOU token mint
                let rewards_token_mint = next_account_info(account_info_iter)?;
                let rewards_token_account = next_account_info(account_info_iter)?;
                let (reward_token_account_pubkey, _) = find_transit_program_address(
                    program_id,
                    depositor_info.key,
                    rewards_token_mint.key,
                    "lm_reward",
                );
                assert_account_key(rewards_token_account, &reward_token_account_pubkey)?;
                let rewards_fee_account = next_account_info(account_info_iter)?;
                let miner = next_account_info(account_info_iter)?;
                let quarry_rewarder = next_account_info(account_info_iter)?;
                assert_account_key(quarry_rewarder, &rewarder)?;
                let quarry_info = next_account_info(account_info_iter)?;
                let (quarry, _) = cpi::quarry::find_quarry_program_address(
                    staking_program_id_info.key,
                    quarry_rewarder.key,
                    liquidity_mint_info.key,
                );
                assert_account_key(quarry_info, &quarry)?;

                cpi::quarry::claim_rewards(
                    staking_program_id_info.key,
                    mint_wrapper.clone(),
                    mint_wrapper_program.clone(),
                    minter.clone(),
                    rewards_token_mint.clone(),
                    rewards_token_account.clone(),
                    rewards_fee_account.clone(),
                    depositor_authority_info.clone(),
                    miner.clone(),
                    quarry_info.clone(),
                    quarry_rewarder.clone(),
                    &[signers_seeds.as_ref()],
                )?;

                let redeemer_program_id_info = next_account_info(account_info_iter)?;
                let redeemer_info = next_account_info(account_info_iter)?;
                let redemption_vault_info = next_account_info(account_info_iter)?;

                cpi::quarry::redeem_all_tokens(
                    redeemer_program_id_info.key,
                    redeemer_info.clone(),
                    rewards_token_mint.clone(),
                    rewards_token_account.clone(),
                    redemption_vault_info.clone(),
                    reward_accounts.reward_transit_info.clone(),
                    depositor_authority_info.clone(),
                    &[signers_seeds.as_ref()],
                )?;
            }
            MiningType::None => {}
        };

        let mut fill_itr = vec![reward_accounts];

        if let Some(accounts) = fill_sub_rewards_accounts {
            fill_itr.push(accounts);
        }

        fill_itr.iter().try_for_each(|reward_accounts| {
            let reward_transit_account =
                Account::unpack(&reward_accounts.reward_transit_info.data.borrow())?;

            everlend_rewards::cpi::fill_vault(
                eld_reward_program_id.key,
                reward_pool_info.clone(),
                reward_accounts.reward_mint_info.clone(),
                reward_accounts.fee_account_info.clone(),
                reward_accounts.vault_info.clone(),
                reward_accounts.reward_transit_info.clone(),
                depositor_authority_info.clone(),
                reward_transit_account.amount,
                &[signers_seeds.as_ref()],
            )
        })?;

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
            DepositorInstruction::Init { rebalance_executor } => {
                msg!("DepositorInstruction: Init");
                Self::init(program_id, accounts, rebalance_executor)
            }

            DepositorInstruction::CreateTransit { seed } => {
                msg!("DepositorInstruction: CreateTransit");
                Self::create_transit(program_id, seed, accounts)
            }

            DepositorInstruction::StartRebalancing { refresh_income } => {
                msg!("DepositorInstruction: StartRebalancing");
                Self::start_rebalancing(program_id, accounts, refresh_income)
            }

            DepositorInstruction::SetRebalancing {
                amount_to_distribute,
                distributed_liquidity,
                distribution_array,
            } => {
                msg!("DepositorInstruction: ResetRebalancing");
                Self::set_rebalancing(
                    program_id,
                    accounts,
                    amount_to_distribute,
                    distributed_liquidity,
                    distribution_array,
                )
            }

            DepositorInstruction::Deposit => {
                msg!("DepositorInstruction: Deposit");
                Self::deposit(program_id, accounts)
            }

            DepositorInstruction::Withdraw => {
                let account_info_iter = &mut accounts.iter().enumerate();
                msg!("DepositorInstruction: Withdraw");
                WithdrawContext::new(program_id, account_info_iter)?
                    .process(program_id, account_info_iter)
            }

            DepositorInstruction::MigrateDepositor => {
                msg!("DepositorInstruction: MigrateDepositor");
                Self::migrate_depositor(program_id, accounts)
            }

            DepositorInstruction::InitMiningAccount { mining_type } => {
                msg!("DepositorInstruction: InitMiningAccount");
                Self::init_mining_account(program_id, accounts, mining_type)
            }

            DepositorInstruction::ClaimMiningReward { with_subrewards } => {
                msg!("DepositorInstruction: ClaimMiningReward");
                Self::claim_mining_reward(program_id, accounts, with_subrewards)
            }

            DepositorInstruction::RefreshMMIncomes => {
                let account_info_iter = &mut accounts.iter().enumerate();
                msg!("DepositorInstruction: RefreshMMIncomes");
                RefreshMMIncomesContext::new(program_id, account_info_iter)?
                    .process(program_id, account_info_iter)
            }
        }
    }
}
