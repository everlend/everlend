//! Program state processor

use std::cmp::Ordering;

use crate::{
    find_rebalancing_program_address, find_transit_program_address,
    instruction::DepositorInstruction,
    state::{
        Depositor, InitDepositorParams, InitRebalancingParams, Rebalancing, RebalancingOperation,
        RESERVED_LEAKED,
    },
    utils::{money_market_deposit, money_market_redeem},
};
use borsh::BorshDeserialize;
use everlend_general_pool::state::Pool;
use everlend_liquidity_oracle::{
    find_liquidity_oracle_token_distribution_program_address, state::TokenDistribution,
};
use everlend_registry::state::RegistryConfig;
use everlend_utils::{
    assert_account_key, assert_owned_by, assert_rent_exempt, assert_uninitialized, cpi,
    find_program_address, EverlendError,
};
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

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Process Init instruction
    pub fn init(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let depositor_info = next_account_info(account_info_iter)?;
        let general_pool_market_info = next_account_info(account_info_iter)?;
        let income_pool_market_info = next_account_info(account_info_iter)?;
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;

        assert_rent_exempt(rent, depositor_info)?;
        assert_owned_by(depositor_info, program_id)?;
        // TODO: replace to getting id from config program
        assert_owned_by(general_pool_market_info, &everlend_general_pool::id())?;
        assert_owned_by(income_pool_market_info, &everlend_income_pools::id())?;
        assert_owned_by(liquidity_oracle_info, &everlend_liquidity_oracle::id())?;

        // Get depositor state
        let mut depositor = Depositor::unpack_unchecked(&depositor_info.data.borrow())?;
        assert_uninitialized(&depositor)?;

        depositor.init(InitDepositorParams {
            general_pool_market: *general_pool_market_info.key,
            income_pool_market: *income_pool_market_info.key,
            liquidity_oracle: *liquidity_oracle_info.key,
        });

        Depositor::pack(depositor, *depositor_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process CreateTransit instruction
    pub fn create_transit(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
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
            find_transit_program_address(program_id, depositor_info.key, mint_info.key);
        assert_account_key(transit_info, &transit_pubkey)?;

        let signers_seeds = &[
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
    pub fn start_rebalancing(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let registry_config_info = next_account_info(account_info_iter)?;
        let registry_config = RegistryConfig::unpack(&registry_config_info.data.borrow())?;

        let depositor_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;
        let rebalancing_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;

        let general_pool_market_info = next_account_info(account_info_iter)?;
        let general_pool_market_authority_info = next_account_info(account_info_iter)?;
        let general_pool_info = next_account_info(account_info_iter)?;
        let general_pool_token_account_info = next_account_info(account_info_iter)?;
        let general_pool_borrow_authority_info = next_account_info(account_info_iter)?;

        let liquidity_transit_info = next_account_info(account_info_iter)?;

        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let token_distribution_info = next_account_info(account_info_iter)?;
        let from_info = next_account_info(account_info_iter)?;

        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;
        let _liquidity_oracle_program_info = next_account_info(account_info_iter)?;
        let _general_pool_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(registry_config_info, &everlend_registry::id())?;
        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(token_distribution_info, &everlend_liquidity_oracle::id())?;
        assert_owned_by(general_pool_info, &everlend_general_pool::id())?;

        // Get depositor state
        let depositor = Depositor::unpack(&depositor_info.data.borrow())?;

        assert_account_key(general_pool_market_info, &depositor.general_pool_market)?;
        assert_account_key(liquidity_oracle_info, &depositor.liquidity_oracle)?;

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

        // Check token distribution pubkey
        let (token_distribution_pubkey, _) =
            find_liquidity_oracle_token_distribution_program_address(
                &everlend_liquidity_oracle::id(),
                liquidity_oracle_info.key,
                mint_info.key,
            );
        assert_account_key(token_distribution_info, &token_distribution_pubkey)?;

        let new_token_distribution =
            TokenDistribution::unpack(&token_distribution_info.data.borrow())?;

        // Get general pool state to calculate total amount
        let general_pool = Pool::unpack(&general_pool_info.data.borrow())?;
        assert_account_key(general_pool_market_info, &general_pool.pool_market)?;
        assert_account_key(general_pool_token_account_info, &general_pool.token_account)?;

        // Calculate total liquidity supply
        let general_pool_token_account =
            Account::unpack_unchecked(&general_pool_token_account_info.data.borrow())?;

        let liquidity_transit_supply = Account::unpack(&liquidity_transit_info.data.borrow())?
            .amount
            .saturating_sub(rebalancing.compute_unused_liquidity()?);

        let available_transit_liquidity = liquidity_transit_supply.saturating_sub(RESERVED_LEAKED);

        // How many lamports do we need to borrow to transit account for leaked
        // N from 0 to 10
        let leaked_amount = RESERVED_LEAKED.saturating_sub(available_transit_liquidity);

        // TODO: Move this math to state

        let new_distributed_liquidity = general_pool_token_account
            .amount
            .checked_add(rebalancing.distributed_liquidity)
            .ok_or(EverlendError::MathOverflow)?
            .checked_sub(leaked_amount)
            .ok_or(EverlendError::MathOverflow)?;

        let amount = (new_distributed_liquidity.saturating_sub(rebalancing.distributed_liquidity)
            as i64)
            .checked_add(leaked_amount as i64)
            .ok_or(EverlendError::MathOverflow)?
            .checked_sub(liquidity_transit_supply as i64)
            .ok_or(EverlendError::MathOverflow)?;

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
        rebalancing.compute(
            &registry_config,
            new_token_distribution,
            new_distributed_liquidity,
        )?;

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

        let mm_pool_market_info = next_account_info(account_info_iter)?;
        let mm_pool_market_authority_info = next_account_info(account_info_iter)?;
        let mm_pool_info = next_account_info(account_info_iter)?;
        let mm_pool_token_account_info = next_account_info(account_info_iter)?;
        let mm_pool_collateral_transit_info = next_account_info(account_info_iter)?;
        let mm_pool_collateral_mint_info = next_account_info(account_info_iter)?;

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

        let registry_config = RegistryConfig::unpack(&registry_config_info.data.borrow())?;

        let mut rebalancing = Rebalancing::unpack(&rebalancing_info.data.borrow())?;
        assert_account_key(depositor_info, &rebalancing.depositor)?;
        assert_account_key(liquidity_mint_info, &rebalancing.mint)?;

        if rebalancing.is_completed() {
            return Err(EverlendError::RebalancingIsCompleted.into());
        }

        // Create depositor authority account
        let (depositor_authority_pubkey, bump_seed) =
            find_program_address(program_id, depositor_info.key);
        assert_account_key(depositor_authority_info, &depositor_authority_pubkey)?;

        let signers_seeds = &[&depositor_info.key.to_bytes()[..32], &[bump_seed]];

        let step = rebalancing.next_step();

        if registry_config.money_market_program_ids[usize::from(step.money_market_index)]
            != *money_market_program_info.key
        {
            return Err(EverlendError::InvalidRebalancingMoneyMarket.into());
        }

        msg!("Deposit to Money market");
        money_market_deposit(
            &registry_config,
            money_market_program_info.clone(),
            liquidity_transit_info.clone(),
            liquidity_mint_info.clone(),
            collateral_transit_info.clone(),
            collateral_mint_info.clone(),
            depositor_authority_info.clone(),
            account_info_iter,
            clock_info.clone(),
            step.liquidity_amount,
            &[signers_seeds],
        )?;

        let collateral_amount =
            Account::unpack_unchecked(&collateral_transit_info.data.borrow())?.amount;

        msg!("Collect collateral tokens to MM Pool");
        everlend_ulp::cpi::deposit(
            mm_pool_market_info.clone(),
            mm_pool_market_authority_info.clone(),
            mm_pool_info.clone(),
            collateral_transit_info.clone(),
            mm_pool_collateral_transit_info.clone(),
            mm_pool_token_account_info.clone(),
            mm_pool_collateral_mint_info.clone(),
            depositor_authority_info.clone(),
            collateral_amount,
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

        let mm_pool_market_info = next_account_info(account_info_iter)?;
        let mm_pool_market_authority_info = next_account_info(account_info_iter)?;
        let mm_pool_info = next_account_info(account_info_iter)?;
        let mm_pool_token_account_info = next_account_info(account_info_iter)?;
        let mm_pool_collateral_transit_info = next_account_info(account_info_iter)?;
        let mm_pool_collateral_mint_info = next_account_info(account_info_iter)?;

        let collateral_transit_info = next_account_info(account_info_iter)?;
        let collateral_mint_info = next_account_info(account_info_iter)?;
        let liquidity_transit_info = next_account_info(account_info_iter)?;
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

        let registry_config = RegistryConfig::unpack(&registry_config_info.data.borrow())?;

        let mut rebalancing = Rebalancing::unpack(&rebalancing_info.data.borrow())?;
        assert_account_key(depositor_info, &rebalancing.depositor)?;
        assert_account_key(liquidity_mint_info, &rebalancing.mint)?;

        if rebalancing.is_completed() {
            return Err(EverlendError::RebalancingIsCompleted.into());
        }

        // Create depositor authority account
        let (depositor_authority_pubkey, bump_seed) =
            find_program_address(program_id, depositor_info.key);
        assert_account_key(depositor_authority_info, &depositor_authority_pubkey)?;

        let signers_seeds = &[&depositor_info.key.to_bytes()[..32], &[bump_seed]];

        let step = rebalancing.next_step();

        if registry_config.money_market_program_ids[usize::from(step.money_market_index)]
            != *money_market_program_info.key
        {
            return Err(EverlendError::InvalidRebalancingMoneyMarket.into());
        }

        let liquidity_transit_supply =
            Account::unpack(&liquidity_transit_info.data.borrow())?.amount;

        msg!("Withdraw collateral tokens from MM Pool");
        everlend_ulp::cpi::withdraw(
            mm_pool_market_info.clone(),
            mm_pool_market_authority_info.clone(),
            mm_pool_info.clone(),
            mm_pool_collateral_transit_info.clone(),
            collateral_transit_info.clone(),
            mm_pool_token_account_info.clone(),
            mm_pool_collateral_mint_info.clone(),
            depositor_authority_info.clone(),
            step.collateral_amount.unwrap(),
            &[signers_seeds],
        )?;

        msg!("Redeem from Money market");
        money_market_redeem(
            &registry_config,
            money_market_program_info.clone(),
            collateral_transit_info.clone(),
            collateral_mint_info.clone(),
            liquidity_transit_info.clone(),
            liquidity_mint_info.clone(),
            depositor_authority_info.clone(),
            account_info_iter,
            clock_info.clone(),
            step.collateral_amount.unwrap(),
            &[signers_seeds],
        )?;

        let received_amount = Account::unpack(&liquidity_transit_info.data.borrow())?
            .amount
            .checked_sub(liquidity_transit_supply)
            .ok_or(EverlendError::MathOverflow)?;
        msg!("Received amount: {}", received_amount);

        // TODO: Received liquidity amount may be less
        // https://blog.neodyme.io/posts/lending_disclosure
        let income_amount: i64 = (received_amount as i64)
            .checked_sub(step.liquidity_amount as i64)
            .ok_or(EverlendError::MathOverflow)?;

        msg!("Step amount: {}", step.liquidity_amount);
        msg!("Income amount: {}", income_amount);

        // Deposit to income pool if income amount > 0
        if income_amount > 0 {
            everlend_income_pools::cpi::deposit(
                income_pool_market_info.clone(),
                income_pool_info.clone(),
                liquidity_transit_info.clone(),
                income_pool_token_account_info.clone(),
                depositor_authority_info.clone(),
                income_amount as u64,
                &[signers_seeds],
            )?;
        } else {
            msg!("Leaked!");
        }

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

            DepositorInstruction::CreateTransit => {
                msg!("DepositorInstruction: CreateTransit");
                Self::create_transit(program_id, accounts)
            }

            DepositorInstruction::StartRebalancing => {
                msg!("DepositorInstruction: StartRebalancing");
                Self::start_rebalancing(program_id, accounts)
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
