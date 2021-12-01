//! Program state processor

use crate::{
    find_transit_program_address,
    instruction::DepositorInstruction,
    state::Depositor,
    utils::{money_market_deposit, money_market_redeem},
};
use borsh::BorshDeserialize;
use everlend_utils::{
    assert_account_key, assert_owned_by, assert_rent_exempt, assert_uninitialized, cpi,
    find_program_address,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Process Init instruction
    pub fn init(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let depositor_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;

        assert_rent_exempt(rent, depositor_info)?;
        assert_owned_by(depositor_info, program_id)?;

        // Get depositor state
        let mut depositor = Depositor::unpack_unchecked(&depositor_info.data.borrow())?;
        assert_uninitialized(&depositor)?;

        depositor.init();

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

    /// Process Deposit instruction
    pub fn deposit(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let depositor_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;

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
        let _everlend_ulp_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        let money_market_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(depositor_info, program_id)?;

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
            amount,
            &[signers_seeds],
        )?;

        Ok(())
    }

    /// Process Withdraw instruction
    pub fn withdraw(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();

        let depositor_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;

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
        let _everlend_ulp_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        let money_market_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(depositor_info, program_id)?;

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
