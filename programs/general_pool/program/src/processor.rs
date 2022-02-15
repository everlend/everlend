//! Program state processor

use crate::{find_pool_borrow_authority_program_address, find_pool_program_address, find_transit_program_address, find_withdrawal_requests_program_address, instruction::LiquidityPoolsInstruction, state::{
    InitPoolBorrowAuthorityParams, InitPoolMarketParams, InitPoolParams, Pool,
    PoolBorrowAuthority, PoolMarket, WithdrawalRequests, WithdrawalRequest, TOTAL_WITHDRAW_REQUEST
}, utils::*};
use borsh::BorshDeserialize;
use everlend_utils::{
    assert_account_key, assert_owned_by, assert_rent_exempt, assert_signer, assert_uninitialized,
    cpi, find_program_address, EverlendError,
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
use spl_token::state::Mint;
use crate::state::InitWithdrawalRequestsParams;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Process InitPoolMarket instruction
    pub fn init_pool_market(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;

        assert_rent_exempt(rent, pool_market_info)?;

        assert_owned_by(pool_market_info, program_id)?;

        // Get pool market state
        let mut pool_market = PoolMarket::unpack_unchecked(&pool_market_info.data.borrow())?;
        assert_uninitialized(&pool_market)?;

        pool_market.init(InitPoolMarketParams {
            manager: *manager_info.key,
        });

        PoolMarket::pack(pool_market, *pool_market_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process CreatePool instruction
    pub fn create_pool(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let withdrawal_requests_info = next_account_info(account_info_iter)?;
        let token_mint_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let transit_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        assert_owned_by(pool_market_info, program_id)?;

        // Get pool market state
        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;

        assert_account_key(manager_info, &pool_market.manager)?;

        let token_mint = Mint::unpack(&token_mint_info.data.borrow())?;

        // Create pool account
        let (pool_pubkey, pool_bump_seed) =
            find_pool_program_address(program_id, pool_market_info.key, token_mint_info.key);
        assert_account_key(pool_info, &pool_pubkey)?;

        let pool_signers_seeds = &[
            &pool_market_info.key.to_bytes()[..32],
            &token_mint_info.key.to_bytes()[..32],
            &[pool_bump_seed],
        ];

        cpi::system::create_account::<Pool>(
            program_id,
            manager_info.clone(),
            pool_info.clone(),
            &[pool_signers_seeds],
            rent,
        )?;

        // Get pool state
        let mut pool = Pool::unpack_unchecked(&pool_info.data.borrow())?;
        assert_uninitialized(&pool)?;

        // Initialize token account for spl token
        cpi::spl_token::initialize_account(
            token_account_info.clone(),
            token_mint_info.clone(),
            pool_market_authority_info.clone(),
            rent_info.clone(),
        )?;

        // Initialize mint (token) for pool
        cpi::spl_token::initialize_mint(
            pool_mint_info.clone(),
            pool_market_authority_info.clone(),
            rent_info.clone(),
            token_mint.decimals,
        )?;

        // Create transit account for SPL program
        let (transit_pubkey, transit_bump_seed) = find_transit_program_address(
            program_id,
            &pool_market_info.key,
            &pool_mint_info.key,
        );
        assert_account_key(transit_info, &transit_pubkey)?;

        let transit_signers_seeds = &[
            "transit".as_bytes(),
            &pool_market_info.key.to_bytes()[..32],
            &pool_mint_info.key.to_bytes()[..32],
            &[transit_bump_seed],
        ];

        cpi::system::create_account::<spl_token::state::Account>(
            &spl_token::id(),
            manager_info.clone(),
            transit_info.clone(),
            &[transit_signers_seeds],
            rent,
        )?;

        // Initialize transit token account for spl token
        cpi::spl_token::initialize_account(
            transit_info.clone(),
            pool_mint_info.clone(),
            pool_market_authority_info.clone(),
            rent_info.clone(),
        )?;

        // Check withdraw requests account
        let (withdrawal_requests_pubkey, withdrawal_bump_seed) =
            find_withdrawal_requests_program_address(
                program_id,
                pool_market_info.key,
                token_mint_info.key,
            );
        assert_account_key(withdrawal_requests_info, &withdrawal_requests_pubkey)?;

        let withdraw_signers_seeds = &[
            "withdrawals".as_bytes(),
            &pool_market_info.key.to_bytes()[..32],
            &token_mint_info.key.to_bytes()[..32],
            &[withdrawal_bump_seed],
        ];

        cpi::system::create_account::<WithdrawalRequests>(
            program_id,
            manager_info.clone(),
            withdrawal_requests_info.clone(),
            &[withdraw_signers_seeds],
            rent,
        )?;

        let mut withdrawal_requests =
            WithdrawalRequests::unpack_unchecked(&withdrawal_requests_info.data.borrow())?;
        assert_uninitialized(&withdrawal_requests)?;

        withdrawal_requests.init(InitWithdrawalRequestsParams{
            pool: *pool_info.key,
            mint: *token_mint_info.key,
        });

        WithdrawalRequests::pack(withdrawal_requests, *withdrawal_requests_info.data.borrow_mut())?;

        pool.init(InitPoolParams {
            pool_market: *pool_market_info.key,
            token_mint: *token_mint_info.key,
            token_account: *token_account_info.key,
            pool_mint: *pool_mint_info.key,
        });

        Pool::pack(pool, *pool_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process CreatePoolBorrowAuthority instruction
    pub fn create_pool_borrow_authority(
        program_id: &Pubkey,
        share_allowed: u16,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let borrow_authority_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;

        // Get pool market state
        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
        assert_account_key(manager_info, &pool_market.manager)?;

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;

        // Create pool borrow authority account
        let (pool_borrow_authority_pubkey, bump_seed) = find_pool_borrow_authority_program_address(
            program_id,
            pool_info.key,
            borrow_authority_info.key,
        );
        assert_account_key(pool_borrow_authority_info, &pool_borrow_authority_pubkey)?;

        let signers_seeds = &[
            &pool_info.key.to_bytes()[..32],
            &borrow_authority_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        cpi::system::create_account::<PoolBorrowAuthority>(
            program_id,
            manager_info.clone(),
            pool_borrow_authority_info.clone(),
            &[signers_seeds],
            rent,
        )?;

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack_unchecked(&pool_borrow_authority_info.data.borrow())?;
        assert_uninitialized(&pool_borrow_authority)?;

        pool_borrow_authority.init(InitPoolBorrowAuthorityParams {
            pool: *pool_info.key,
            borrow_authority: *borrow_authority_info.key,
            share_allowed,
        });

        PoolBorrowAuthority::pack(
            pool_borrow_authority,
            *pool_borrow_authority_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    /// Process UpdatePoolBorrowAuthority instruction
    pub fn update_pool_borrow_authority(
        program_id: &Pubkey,
        share_allowed: u16,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        assert_owned_by(pool_borrow_authority_info, program_id)?;

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;

        pool_borrow_authority.update_share_allowed(share_allowed);

        PoolBorrowAuthority::pack(
            pool_borrow_authority,
            *pool_borrow_authority_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    /// Process DeletePoolBorrowAuthority instruction
    pub fn delete_pool_borrow_authority(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let receiver_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        assert_owned_by(pool_borrow_authority_info, program_id)?;

        // Get pool borrow authority state to check initialized
        PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;

        let receiver_starting_lamports = receiver_info.lamports();
        let pool_borrow_authority_lamports = pool_borrow_authority_info.lamports();

        **pool_borrow_authority_info.lamports.borrow_mut() = 0;
        **receiver_info.lamports.borrow_mut() = receiver_starting_lamports
            .checked_add(pool_borrow_authority_lamports)
            .ok_or(EverlendError::MathOverflow)?;

        PoolBorrowAuthority::pack(
            Default::default(),
            *pool_borrow_authority_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    /// Process Deposit instruction
    pub fn deposit(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(user_transfer_authority_info)?;

        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;

        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;
        assert_account_key(pool_mint_info, &pool.pool_mint)?;

        let total_incoming =
            total_pool_amount(token_account_info.clone(), pool.total_amount_borrowed)?;
        let total_minted = Mint::unpack_unchecked(&pool_mint_info.data.borrow())?.supply;

        let mint_amount = if total_incoming == 0 || total_minted == 0 {
            amount
        } else {
            (amount as u128)
                .checked_mul(total_minted as u128)
                .ok_or(ProgramError::InvalidArgument)?
                .checked_div(total_incoming as u128)
                .ok_or(ProgramError::InvalidArgument)? as u64
        };

        // Transfer token from source to token account
        cpi::spl_token::transfer(
            source_info.clone(),
            token_account_info.clone(),
            user_transfer_authority_info.clone(),
            amount,
            &[],
        )?;

        let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
        let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

        // Mint to destination pool token
        cpi::spl_token::mint_to(
            pool_mint_info.clone(),
            destination_info.clone(),
            pool_market_authority_info.clone(),
            mint_amount,
            &[signers_seeds],
        )?;

        Ok(())
    }

    /// Process Withdraw instruction
    pub fn withdraw(program_id: &Pubkey, index: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let withdrawal_requests_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let transit_collateral_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;

        // Check collateral token transit account
        let (collateral_transit_pubkey, _) = find_transit_program_address(program_id, pool_market_info.key,pool_mint_info.key);
        assert_account_key(transit_collateral_info, &collateral_transit_pubkey)?;

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;

        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;

        let (withdrawal_requests_pubkey, _) =
            find_withdrawal_requests_program_address(
                program_id,
                pool_market_info.key,
                &pool.token_mint,
            );
        assert_account_key(withdrawal_requests_info, &withdrawal_requests_pubkey)?;

        if withdrawal_requests_info.lamports() == 0 {
            return Err(ProgramError::UninitializedAccount)
        }

        let mut withdrawal_requests =
            WithdrawalRequests::unpack_unchecked(&withdrawal_requests_info.data.borrow())?;

        assert_owned_by(withdrawal_requests_info, program_id)?;
        assert_account_key(pool_info, &withdrawal_requests.pool)?;
        assert_account_key(pool_info, &withdrawal_requests.pool)?;

        if withdrawal_requests.mint != pool.token_mint{
          return Err(ProgramError::InvalidArgument)
        }

        // Requests que is empty
        if withdrawal_requests.request.len() == 0 {
            return Err(EverlendError::WithdrawRequestsListIsEmpty.into())
        }

        if index > (withdrawal_requests.request.len() - 1) as u64{
            return Err(ProgramError::InvalidArgument)
        }

        let request = withdrawal_requests.request[index as usize];

        assert_account_key(destination_info,&request.destination)?;

        let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
        let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];
        // Transfer from token account to destination
        cpi::spl_token::transfer(
            token_account_info.clone(),
            destination_info.clone(),
            pool_market_authority_info.clone(),
            request.liquidity_amount,
            &[signers_seeds],
        )?;

        // Burn from transit collateral pool token
        cpi::spl_token::burn(
            pool_mint_info.clone(),
            transit_collateral_info.clone(),
            pool_market_authority_info.clone(),
            request.collateral_amount,
            &[signers_seeds],
        )?;

        withdrawal_requests.request.remove(index as usize);
        withdrawal_requests.liquidity_supply -= request.liquidity_amount;

        WithdrawalRequests::pack(withdrawal_requests, *withdrawal_requests_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process Withdraw request instruction
    pub fn withdraw_request(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let withdrawal_requests_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let transit_collateral_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(user_transfer_authority_info)?;

        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;

        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;
        assert_account_key(pool_mint_info, &pool.pool_mint)?;

        // Check collateral token transit account
        let (collateral_transit_pubkey, _) = find_transit_program_address(program_id, pool_market_info.key,pool_mint_info.key);
        assert_account_key(transit_collateral_info, &collateral_transit_pubkey)?;

        // Check withdraw requests account
        assert_owned_by(withdrawal_requests_info, program_id)?;

        let (withdrawal_requests_pubkey, _) =
            find_withdrawal_requests_program_address(
                program_id,
                pool_market_info.key,
                &pool.token_mint,
            );
        assert_account_key(withdrawal_requests_info, &withdrawal_requests_pubkey)?;

        if withdrawal_requests_info.lamports() == 0 {
            return Err(ProgramError::UninitializedAccount)
        }

        // Get withdrawals account
        let mut withdrawal_requests =
            WithdrawalRequests::unpack_unchecked(&withdrawal_requests_info.data.borrow())?;

        assert_owned_by(withdrawal_requests_info, program_id)?;
        assert_account_key(pool_info, &withdrawal_requests.pool)?;
        if withdrawal_requests.mint != pool.token_mint{
            return Err(ProgramError::InvalidArgument)
        }

        // // TODO change error
        if withdrawal_requests.request.len() == TOTAL_WITHDRAW_REQUEST {
            return Err(ProgramError::InvalidAccountData);
        }

        let total_incoming =
            total_pool_amount(token_account_info.clone(), pool.total_amount_borrowed)?;
        let total_minted = Mint::unpack_unchecked(&pool_mint_info.data.borrow())?.supply;

        let withdraw_amount = (amount as u128)
            .checked_mul(total_incoming as u128)
            .ok_or(ProgramError::InvalidArgument)?
            .checked_div(total_minted as u128)
            .ok_or(ProgramError::InvalidArgument)? as u64;

        // Transfer
        cpi::spl_token::transfer(
            source_info.clone(),
            transit_collateral_info.clone(),
            user_transfer_authority_info.clone(),
            amount,
            &[],
        )?;

        withdrawal_requests.request.push(WithdrawalRequest{
            destination: *destination_info.key,
            liquidity_amount: withdraw_amount,
            collateral_amount: amount,
        });

        withdrawal_requests.liquidity_supply += withdraw_amount;

        WithdrawalRequests::pack(withdrawal_requests, *withdrawal_requests_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process Borrow instruction
    pub fn borrow(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let borrow_authority_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(borrow_authority_info)?;

        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(pool_borrow_authority_info, program_id)?;

        // Get pool state
        let mut pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;
        assert_account_key(pool_info, &pool_borrow_authority.pool)?;
        assert_account_key(
            borrow_authority_info,
            &pool_borrow_authority.borrow_authority,
        )?;

        pool_borrow_authority.borrow(amount)?;
        pool_borrow_authority.check_amount_allowed(total_pool_amount(
            token_account_info.clone(),
            pool.total_amount_borrowed,
        )?)?;
        pool.borrow(amount)?;

        // Checks...

        PoolBorrowAuthority::pack(
            pool_borrow_authority,
            *pool_borrow_authority_info.data.borrow_mut(),
        )?;
        Pool::pack(pool, *pool_info.data.borrow_mut())?;

        let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
        let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

        // Transfer from token account to destination borrower
        cpi::spl_token::transfer(
            token_account_info.clone(),
            destination_info.clone(),
            pool_market_authority_info.clone(),
            amount,
            &[signers_seeds],
        )?;

        Ok(())
    }

    /// Process Repay instruction
    pub fn repay(
        program_id: &Pubkey,
        amount: u64,
        interest_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(user_transfer_authority_info)?;

        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(pool_borrow_authority_info, program_id)?;

        // Get pool state
        let mut pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;
        assert_account_key(pool_info, &pool_borrow_authority.pool)?;

        pool_borrow_authority.repay(amount)?;
        pool.repay(amount)?;

        // Checks...

        PoolBorrowAuthority::pack(
            pool_borrow_authority,
            *pool_borrow_authority_info.data.borrow_mut(),
        )?;
        Pool::pack(pool, *pool_info.data.borrow_mut())?;

        // Transfer from source to token account
        cpi::spl_token::transfer(
            source_info.clone(),
            token_account_info.clone(),
            user_transfer_authority_info.clone(),
            amount + interest_amount,
            &[],
        )?;

        Ok(())
    }

    /// Instruction processing router
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = LiquidityPoolsInstruction::try_from_slice(input)?;

        match instruction {
            LiquidityPoolsInstruction::InitPoolMarket => {
                msg!("LiquidityPoolsInstruction: InitPoolMarket");
                Self::init_pool_market(program_id, accounts)
            }

            LiquidityPoolsInstruction::CreatePool => {
                msg!("LiquidityPoolsInstruction: CreatePool");
                Self::create_pool(program_id, accounts)
            }

            LiquidityPoolsInstruction::CreatePoolBorrowAuthority { share_allowed } => {
                msg!("LiquidityPoolsInstruction: CreatePoolBorrowAuthority");
                Self::create_pool_borrow_authority(program_id, share_allowed, accounts)
            }

            LiquidityPoolsInstruction::UpdatePoolBorrowAuthority { share_allowed } => {
                msg!("LiquidityPoolsInstruction: UpdatePoolBorrowAuthority");
                Self::update_pool_borrow_authority(program_id, share_allowed, accounts)
            }

            LiquidityPoolsInstruction::DeletePoolBorrowAuthority => {
                msg!("LiquidityPoolsInstruction: DeletePoolBorrowAuthority");
                Self::delete_pool_borrow_authority(program_id, accounts)
            }

            LiquidityPoolsInstruction::Deposit { amount } => {
                msg!("LiquidityPoolsInstruction: Deposit");
                Self::deposit(program_id, amount, accounts)
            }

            LiquidityPoolsInstruction::Withdraw { index } => {
                msg!("LiquidityPoolsInstruction: Withdraw");
                Self::withdraw(program_id, index, accounts)
            }

            LiquidityPoolsInstruction::WithdrawRequest { amount } => {
                msg!("LiquidityPoolsInstruction: WithdrawRequest");
                Self::withdraw_request(program_id, amount, accounts)
            }

            LiquidityPoolsInstruction::Borrow { amount } => {
                msg!("LiquidityPoolsInstruction: Borrow");
                Self::borrow(program_id, amount, accounts)
            }

            LiquidityPoolsInstruction::Repay {
                amount,
                interest_amount,
            } => {
                msg!("LiquidityPoolsInstruction: Repay");
                Self::repay(program_id, amount, interest_amount, accounts)
            }
        }
    }
}
