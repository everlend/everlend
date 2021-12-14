//! Program state processor

use crate::{
    find_rebalancing_program_address, find_transit_program_address,
    instruction::DepositorInstruction,
    state::{
        Depositor, InitDepositorParams, InitRebalancingParams, Rebalancing, RebalancingOperation,
    },
    utils::{money_market_deposit, money_market_redeem},
};
use borsh::BorshDeserialize;
use everlend_liquidity_oracle::{
    find_liquidity_oracle_token_distribution_program_address, state::TokenDistribution,
};
use everlend_ulp::state::Pool;
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
        let pool_market_info = next_account_info(account_info_iter)?;
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;

        assert_rent_exempt(rent, depositor_info)?;
        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(pool_market_info, &everlend_ulp::id())?;
        assert_owned_by(liquidity_oracle_info, &everlend_liquidity_oracle::id())?;

        // Get depositor state
        let mut depositor = Depositor::unpack_unchecked(&depositor_info.data.borrow())?;
        assert_uninitialized(&depositor)?;

        depositor.init(InitDepositorParams {
            pool_market: *pool_market_info.key,
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
        let depositor_info = next_account_info(account_info_iter)?;
        let rebalancing_info = next_account_info(account_info_iter)?;
        let mint_info = next_account_info(account_info_iter)?;
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_token_account_info = next_account_info(account_info_iter)?;
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let token_distribution_info = next_account_info(account_info_iter)?;
        let from_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _liquidity_oracle_program_info = next_account_info(account_info_iter)?;
        let _ulp_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(token_distribution_info, &everlend_liquidity_oracle::id())?;
        assert_owned_by(pool_info, &everlend_ulp::id())?;

        // Get depositor state
        let depositor = Depositor::unpack(&depositor_info.data.borrow())?;

        assert_account_key(pool_market_info, &depositor.pool_market)?;
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

        let token_distribution = TokenDistribution::unpack(&token_distribution_info.data.borrow())?;

        // Get pool state to calculate total amount
        let pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(pool_token_account_info, &pool.token_account)?;

        let pool_token_account_amount =
            Account::unpack_unchecked(&pool_token_account_info.data.borrow())?.amount;
        let total_amount = pool_token_account_amount
            .checked_add(pool.total_amount_borrowed)
            .ok_or(EverlendError::MathOverflow)?;

        // Compute rebalancing steps
        rebalancing.compute(token_distribution, total_amount)?;
        msg!("Steps: {:#?}", rebalancing.steps);

        Rebalancing::pack(rebalancing, *rebalancing_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process Deposit instruction
    pub fn deposit(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let depositor_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;
        let rebalancing_info = next_account_info(account_info_iter)?;

        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let pool_token_account_info = next_account_info(account_info_iter)?;

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
        let _everlend_ulp_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        let money_market_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(rebalancing_info, program_id)?;

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

        msg!("Borrow from General Pool");
        everlend_ulp::cpi::borrow(
            pool_market_info.clone(),
            pool_market_authority_info.clone(),
            pool_info.clone(),
            pool_borrow_authority_info.clone(),
            liquidity_transit_info.clone(),
            pool_token_account_info.clone(),
            depositor_authority_info.clone(),
            amount,
            &[signers_seeds],
        )?;

        msg!("Deposit to Money market");
        money_market_deposit(
            money_market_program_info.clone(),
            liquidity_transit_info.clone(),
            liquidity_mint_info.clone(),
            collateral_transit_info.clone(),
            collateral_mint_info.clone(),
            depositor_authority_info.clone(),
            account_info_iter,
            clock_info.clone(),
            amount,
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
            *money_market_program_info.key,
            RebalancingOperation::Deposit,
            amount,
            clock.slot,
        )?;

        Rebalancing::pack(rebalancing, *rebalancing_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process Withdraw instruction
    pub fn withdraw(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let depositor_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;
        let rebalancing_info = next_account_info(account_info_iter)?;

        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let pool_token_account_info = next_account_info(account_info_iter)?;

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
        let _everlend_ulp_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        let money_market_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(depositor_info, program_id)?;
        assert_owned_by(rebalancing_info, program_id)?;

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
            amount,
            &[signers_seeds],
        )?;

        msg!("Redeem from Money market");
        money_market_redeem(
            money_market_program_info.clone(),
            collateral_transit_info.clone(),
            collateral_mint_info.clone(),
            liquidity_transit_info.clone(),
            liquidity_mint_info.clone(),
            depositor_authority_info.clone(),
            account_info_iter,
            clock_info.clone(),
            amount,
            &[signers_seeds],
        )?;

        msg!("Repay to General Pool");
        everlend_ulp::cpi::repay(
            pool_market_info.clone(),
            pool_market_authority_info.clone(),
            pool_info.clone(),
            pool_borrow_authority_info.clone(),
            liquidity_transit_info.clone(),
            pool_token_account_info.clone(),
            depositor_authority_info.clone(),
            amount,
            0,
            &[signers_seeds],
        )?;

        rebalancing.execute_step(
            *money_market_program_info.key,
            RebalancingOperation::Withdraw,
            amount,
            clock.slot,
        )?;

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

            DepositorInstruction::Deposit { amount } => {
                msg!("DepositorInstruction: Deposit");
                Self::deposit(program_id, amount, accounts)
            }

            DepositorInstruction::Withdraw { amount } => {
                msg!("DepositorInstruction: Withdraw");
                Self::withdraw(program_id, amount, accounts)
            }
        }
    }
}
