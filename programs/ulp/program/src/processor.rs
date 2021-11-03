//! Program state processor

use crate::{
    error::LiquidityPoolsError,
    find_pool_borrow_authority_program_address, find_pool_program_address, find_program_address,
    instruction::LiquidityPoolsInstruction,
    state::{
        InitPoolBorrowAuthorityParams, InitPoolMarketParams, InitPoolParams, Pool,
        PoolBorrowAuthority, PoolMarket,
    },
    utils::*,
};
use borsh::BorshDeserialize;
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

        if pool_market_info.owner != program_id {
            msg!("Pool market provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }

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
        let token_mint_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        if !manager_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if pool_market_info.owner != program_id {
            msg!("Pool market provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get pool market state
        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;

        if pool_market.manager != *manager_info.key {
            msg!("Market manager provided does not match manager in the pool market state");
            return Err(ProgramError::InvalidArgument);
        }

        let token_mint = Mint::unpack(&token_mint_info.data.borrow())?;

        // Create pool account
        let (pool_pubkey, bump_seed) =
            find_pool_program_address(program_id, pool_market_info.key, token_mint_info.key);
        if pool_pubkey != *pool_info.key {
            msg!("Pool provided does not match generated pool");
            return Err(ProgramError::InvalidArgument);
        }

        let signers_seeds = &[
            &pool_market_info.key.to_bytes()[..32],
            &token_mint_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        create_account::<Pool>(
            program_id,
            manager_info.clone(),
            pool_info.clone(),
            &[signers_seeds],
            rent,
        )?;

        // Get pool state
        let mut pool = Pool::unpack_unchecked(&pool_info.data.borrow())?;
        assert_uninitialized(&pool)?;

        // Initialize token account for spl token
        spl_initialize_account(
            token_account_info.clone(),
            token_mint_info.clone(),
            pool_market_authority_info.clone(),
            rent_info.clone(),
        )?;

        // Initialize mint (token) for pool
        spl_initialize_mint(
            pool_mint_info.clone(),
            pool_market_authority_info.clone(),
            rent_info.clone(),
            token_mint.decimals,
        )?;

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

        if !manager_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if pool_market_info.owner != program_id {
            msg!("Pool market provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }
        if pool_info.owner != program_id {
            msg!("Pool provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get pool market state
        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
        if pool_market.manager != *manager_info.key {
            msg!("Market manager provided does not match manager in the pool market state");
            return Err(ProgramError::InvalidArgument);
        }

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;
        if pool.pool_market != *pool_market_info.key {
            msg!("Pool market provided does not match pool market in the pool state");
            return Err(ProgramError::InvalidArgument);
        }

        // Create pool borrow authority account
        let (pool_borrow_authority_pubkey, bump_seed) = find_pool_borrow_authority_program_address(
            program_id,
            pool_info.key,
            borrow_authority_info.key,
        );
        if pool_borrow_authority_pubkey != *pool_borrow_authority_info.key {
            msg!("Pool borrow authority provided does not match generated pool borrow authority");
            return Err(ProgramError::InvalidArgument);
        }

        let signers_seeds = &[
            &pool_info.key.to_bytes()[..32],
            &borrow_authority_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        create_account::<PoolBorrowAuthority>(
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

        if !manager_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if pool_borrow_authority_info.owner != program_id {
            msg!("Pool borrow authority provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }

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

        if !manager_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if pool_borrow_authority_info.owner != program_id {
            msg!("Pool borrow authority provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get pool borrow authority state to check initialized
        PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;

        let receiver_starting_lamports = receiver_info.lamports();
        let pool_borrow_authority_lamports = pool_borrow_authority_info.lamports();

        **pool_borrow_authority_info.lamports.borrow_mut() = 0;
        **receiver_info.lamports.borrow_mut() = receiver_starting_lamports
            .checked_add(pool_borrow_authority_lamports)
            .ok_or(LiquidityPoolsError::MathOverflow)?;

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

        if !user_transfer_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if pool_market_info.owner != program_id {
            msg!("Pool market provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }
        if pool_info.owner != program_id {
            msg!("Pool provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;

        if pool.pool_market != *pool_market_info.key {
            msg!("Pool market provided does not match pool market in the pool state");
            return Err(ProgramError::InvalidArgument);
        }

        if pool.token_account != *token_account_info.key {
            msg!("Pool token account does not match the token account provided");
            return Err(ProgramError::InvalidArgument);
        }

        if pool.pool_mint != *pool_mint_info.key {
            msg!("Pool mint does not match the pool mint provided");
            return Err(ProgramError::InvalidArgument);
        }

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
        spl_token_transfer(
            source_info.clone(),
            token_account_info.clone(),
            user_transfer_authority_info.clone(),
            amount,
            &[],
        )?;

        let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
        let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

        // Mint to destination pool token
        spl_token_mint_to(
            pool_mint_info.clone(),
            destination_info.clone(),
            pool_market_authority_info.clone(),
            mint_amount,
            &[signers_seeds],
        )?;

        Ok(())
    }

    /// Process Withdraw instruction
    pub fn withdraw(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
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

        if !user_transfer_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if pool_market_info.owner != program_id {
            msg!("Pool market provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }
        if pool_info.owner != program_id {
            msg!("Pool provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;

        if pool.pool_market != *pool_market_info.key {
            msg!("Pool market provided does not match pool market in the pool state");
            return Err(ProgramError::InvalidArgument);
        }

        if pool.token_account != *token_account_info.key {
            msg!("Pool token account does not match the token account provided");
            return Err(ProgramError::InvalidArgument);
        }

        if pool.pool_mint != *pool_mint_info.key {
            msg!("Pool mint does not match the pool mint provided");
            return Err(ProgramError::InvalidArgument);
        }

        let total_incoming =
            total_pool_amount(token_account_info.clone(), pool.total_amount_borrowed)?;
        let total_minted = Mint::unpack_unchecked(&pool_mint_info.data.borrow())?.supply;

        let transfer_amount = (amount as u128)
            .checked_mul(total_incoming as u128)
            .ok_or(ProgramError::InvalidArgument)?
            .checked_div(total_minted as u128)
            .ok_or(ProgramError::InvalidArgument)? as u64;

        // Burn from soruce pool token
        spl_token_burn(
            pool_mint_info.clone(),
            source_info.clone(),
            user_transfer_authority_info.clone(),
            amount,
            &[],
        )?;

        let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
        let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

        // Transfer from token account to destination
        spl_token_transfer(
            token_account_info.clone(),
            destination_info.clone(),
            pool_market_authority_info.clone(),
            transfer_amount,
            &[signers_seeds],
        )?;

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

        if !borrow_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if pool_market_info.owner != program_id {
            msg!("Pool market provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }
        if pool_info.owner != program_id {
            msg!("Pool provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }
        if pool_borrow_authority_info.owner != program_id {
            msg!("Pool borrow authority provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get pool state
        let mut pool = Pool::unpack(&pool_info.data.borrow())?;
        if pool.pool_market != *pool_market_info.key {
            msg!("Pool market provided does not match pool market in the pool state");
            return Err(ProgramError::InvalidArgument);
        }
        if pool.token_account != *token_account_info.key {
            msg!("Pool token account does not match the token account provided");
            return Err(ProgramError::InvalidArgument);
        }

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;
        if pool_borrow_authority.pool != *pool_info.key {
            msg!("Pool in pool borrow authority state does not match the pool provided");
            return Err(ProgramError::InvalidArgument);
        }
        if pool_borrow_authority.borrow_authority != *borrow_authority_info.key {
            msg!("Pool borrow authority does not match the borrow authority provided");
            return Err(ProgramError::InvalidArgument);
        }

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
        spl_token_transfer(
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

        if !user_transfer_authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if pool_market_info.owner != program_id {
            msg!("Pool market provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }
        if pool_info.owner != program_id {
            msg!("Pool provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }
        if pool_borrow_authority_info.owner != program_id {
            msg!("Pool borrow authority provided is not owned by the program");
            return Err(ProgramError::IncorrectProgramId);
        }

        // Get pool state
        let mut pool = Pool::unpack(&pool_info.data.borrow())?;
        if pool.pool_market != *pool_market_info.key {
            msg!("Pool market provided does not match pool market in the pool state");
            return Err(ProgramError::InvalidArgument);
        }
        if pool.token_account != *token_account_info.key {
            msg!("Pool token account does not match the token account provided");
            return Err(ProgramError::InvalidArgument);
        }

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;
        if pool_borrow_authority.pool != *pool_info.key {
            msg!("Pool in pool borrow authority state does not match the pool provided");
            return Err(ProgramError::InvalidArgument);
        }

        pool_borrow_authority.repay(amount)?;
        pool.repay(amount)?;

        // Checks...

        PoolBorrowAuthority::pack(
            pool_borrow_authority,
            *pool_borrow_authority_info.data.borrow_mut(),
        )?;
        Pool::pack(pool, *pool_info.data.borrow_mut())?;

        // Transfer from source to token account
        spl_token_transfer(
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

            LiquidityPoolsInstruction::Withdraw { amount } => {
                msg!("LiquidityPoolsInstruction: Withdraw");
                Self::withdraw(program_id, amount, accounts)
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
