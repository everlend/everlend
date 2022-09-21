//! Program state processor

use crate::{
    find_pool_program_address, find_safety_fund_token_account_address,
    instruction::IncomePoolsInstruction,
    safety_fund_token_account_seed,
    state::{IncomePool, IncomePoolMarket, InitIncomePoolMarketParams, InitIncomePoolParams},
};
use borsh::BorshDeserialize;
use everlend_general_pool::state::Pool;
use everlend_utils::{
    assert_account_key, assert_owned_by, assert_rent_exempt, assert_signer, assert_uninitialized,
    assert_non_zero_amount, cpi, find_program_address, math, EverlendError,
};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_token::state::Account;

///Income fee 2%
const INCOME_FEE: u64 = 20_000_000;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Process InitPoolMarket instruction
    pub fn init_pool_market(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let general_pool_market_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;

        assert_rent_exempt(rent, pool_market_info)?;

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;
        // TODO: replace to getting id from config program
        assert_owned_by(general_pool_market_info, &everlend_general_pool::id())?;

        // Get pool market state
        let mut pool_market = IncomePoolMarket::unpack_unchecked(&pool_market_info.data.borrow())?;
        assert_uninitialized(&pool_market)?;

        pool_market.init(InitIncomePoolMarketParams {
            manager: *manager_info.key,
            general_pool_market: *general_pool_market_info.key,
        });

        IncomePoolMarket::pack(pool_market, *pool_market_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process CreatePool instruction
    pub fn create_pool(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let token_mint_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;

        // Get pool market state
        let pool_market = IncomePoolMarket::unpack(&pool_market_info.data.borrow())?;

        // Check manager
        assert_account_key(manager_info, &pool_market.manager)?;

        // Create pool account
        let (pool_pubkey, bump_seed) =
            find_pool_program_address(program_id, pool_market_info.key, token_mint_info.key);
        assert_account_key(pool_info, &pool_pubkey)?;

        let signers_seeds = &[
            &pool_market_info.key.to_bytes()[..32],
            &token_mint_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        cpi::system::create_account::<IncomePool>(
            program_id,
            manager_info.clone(),
            pool_info.clone(),
            &[signers_seeds],
            rent,
        )?;

        // Get pool state
        let mut pool = IncomePool::unpack_unchecked(&pool_info.data.borrow())?;
        assert_uninitialized(&pool)?;

        // Initialize token account for spl token
        cpi::spl_token::initialize_account(
            token_account_info.clone(),
            token_mint_info.clone(),
            pool_market_authority_info.clone(),
            rent_info.clone(),
        )?;

        pool.init(InitIncomePoolParams {
            income_pool_market: *pool_market_info.key,
            token_mint: *token_mint_info.key,
            token_account: *token_account_info.key,
        });

        IncomePool::pack(pool, *pool_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process Deposit instruction
    pub fn deposit(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let income_pool_market_info = next_account_info(account_info_iter)?;
        let income_pool_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let income_pool_token_account_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_non_zero_amount(amount)?;
        assert_signer(user_transfer_authority_info)?;

        // Check programs
        assert_owned_by(income_pool_market_info, program_id)?;
        assert_owned_by(income_pool_info, program_id)?;

        let income_pool = IncomePool::unpack(&income_pool_info.data.borrow())?;

        // Check income pool accounts
        assert_account_key(income_pool_market_info, &income_pool.income_pool_market)?;
        assert_account_key(income_pool_token_account_info, &income_pool.token_account)?;

        // Transfer token from source to token account
        cpi::spl_token::transfer(
            source_info.clone(),
            income_pool_token_account_info.clone(),
            user_transfer_authority_info.clone(),
            amount,
            &[],
        )?;

        Ok(())
    }

    /// Process Withdraw instruction
    pub fn withdraw(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let income_pool_market_info = next_account_info(account_info_iter)?;
        let income_pool_info = next_account_info(account_info_iter)?;
        let income_pool_token_account_info = next_account_info(account_info_iter)?;
        let income_pool_market_authority_info = next_account_info(account_info_iter)?;
        let safety_fund_token_account_info = next_account_info(account_info_iter)?;
        let general_pool_info = next_account_info(account_info_iter)?;
        let general_pool_token_account_info = next_account_info(account_info_iter)?;
        let _everlend_general_pool_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        // Check programs
        assert_owned_by(income_pool_market_info, program_id)?;
        assert_owned_by(income_pool_info, program_id)?;
        assert_owned_by(general_pool_info, &everlend_general_pool::id())?;

        let income_pool_market = IncomePoolMarket::unpack(&income_pool_market_info.data.borrow())?;

        let income_pool = IncomePool::unpack(&income_pool_info.data.borrow())?;

        // Check pool accounts
        assert_account_key(income_pool_market_info, &income_pool.income_pool_market)?;
        assert_account_key(income_pool_token_account_info, &income_pool.token_account)?;

        let general_pool = Pool::unpack(&general_pool_info.data.borrow())?;

        // Check general pool
        if general_pool.pool_market != income_pool_market.general_pool_market {
            return Err(ProgramError::InvalidArgument);
        }
        assert_account_key(general_pool_token_account_info, &general_pool.token_account)?;

        let (safety_fund_token_account, _) = find_safety_fund_token_account_address(
            program_id,
            income_pool_market_info.key,
            &general_pool.token_mint,
        );

        assert_account_key(safety_fund_token_account_info, &safety_fund_token_account)?;

        let mut token_amount =
            Account::unpack_unchecked(&income_pool_token_account_info.data.borrow())?.amount;

        assert_non_zero_amount(token_amount)?;

        let safety_fund_amount = math::share_floor(token_amount, INCOME_FEE)?;

        token_amount = token_amount
            .checked_sub(safety_fund_amount)
            .ok_or(EverlendError::MathOverflow)?;

        let (_, bump_seed) = find_program_address(program_id, income_pool_market_info.key);
        let signers_seeds = &[&income_pool_market_info.key.to_bytes()[..32], &[bump_seed]];

        if safety_fund_amount > 0 {
            // Transfer from token account to safety fund
            cpi::spl_token::transfer(
                income_pool_token_account_info.clone(),
                safety_fund_token_account_info.clone(),
                income_pool_market_authority_info.clone(),
                safety_fund_amount,
                &[signers_seeds],
            )?;
        };

        // Transfer from token account to destination
        cpi::spl_token::transfer(
            income_pool_token_account_info.clone(),
            general_pool_token_account_info.clone(),
            income_pool_market_authority_info.clone(),
            token_amount,
            &[signers_seeds],
        )?;

        Ok(())
    }

    /// Process CreateSafetyFundTokenAccount instruction
    pub fn create_safety_fund_token_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let income_pool_market_info = next_account_info(account_info_iter)?;
        let income_pool_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let token_mint_info = next_account_info(account_info_iter)?;
        let safety_fund_token_account_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        assert_owned_by(income_pool_info, program_id)?;

        // Get pool market state
        let income_pool_market = IncomePoolMarket::unpack(&income_pool_market_info.data.borrow())?;

        assert_account_key(manager_info, &income_pool_market.manager)?;

        // Check pool account
        let (income_pool_pubkey, _) =
            find_pool_program_address(program_id, income_pool_market_info.key, token_mint_info.key);
        assert_account_key(income_pool_info, &income_pool_pubkey)?;

        let (safety_fund_token_account_pubkey, bump_seed) = find_safety_fund_token_account_address(
            program_id,
            income_pool_market_info.key,
            token_mint_info.key,
        );

        assert_account_key(
            safety_fund_token_account_info,
            &safety_fund_token_account_pubkey,
        )?;

        let safety_fund_token_account_seed = safety_fund_token_account_seed();
        let signers_seeds = &[
            safety_fund_token_account_seed.as_bytes(),
            &income_pool_market_info.key.to_bytes()[..32],
            &token_mint_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        cpi::system::create_account::<spl_token::state::Account>(
            &spl_token::id(),
            manager_info.clone(),
            safety_fund_token_account_info.clone(),
            &[signers_seeds],
            rent,
        )?;

        // Initialize transit token account for spl token
        cpi::spl_token::initialize_account(
            safety_fund_token_account_info.clone(),
            token_mint_info.clone(),
            pool_market_authority_info.clone(),
            rent_info.clone(),
        )?;

        Ok(())
    }

    /// Process UpdateManager instruction
    pub fn update_manager(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let new_manager_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;
        assert_signer(new_manager_info)?;

        assert_owned_by(pool_market_info, program_id)?;

        let mut pool_market = IncomePoolMarket::unpack(&pool_market_info.data.borrow())?;
        assert_account_key(manager_info, &pool_market.manager)?;

        pool_market.manager = *new_manager_info.key;

        IncomePoolMarket::pack(pool_market, *pool_market_info.data.borrow_mut())?;

        Ok(())
    }

    /// Instruction processing router
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = IncomePoolsInstruction::try_from_slice(input)?;

        match instruction {
            IncomePoolsInstruction::InitPoolMarket => {
                msg!("IncomePoolsInstruction: InitPoolMarket");
                Self::init_pool_market(program_id, accounts)
            }

            IncomePoolsInstruction::CreatePool => {
                msg!("IncomePoolsInstruction: CreatePool");
                Self::create_pool(program_id, accounts)
            }

            IncomePoolsInstruction::Deposit { amount } => {
                msg!("IncomePoolsInstruction: Deposit");
                Self::deposit(program_id, amount, accounts)
            }

            IncomePoolsInstruction::Withdraw => {
                msg!("IncomePoolsInstruction: Withdraw");
                Self::withdraw(program_id, accounts)
            }

            IncomePoolsInstruction::CreateSafetyPoolTokenAccount => {
                msg!("IncomePoolsInstruction: CreateSafetyPoolTokenAccount");
                Self::create_safety_fund_token_account(program_id, accounts)
            }

            IncomePoolsInstruction::UpdateManager => {
                msg!("IncomePoolsInstruction: UpdateManager");
                Self::update_manager(program_id, accounts)
            }
        }
    }
}
