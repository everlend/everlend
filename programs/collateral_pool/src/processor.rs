//! Program state processor

use borsh::BorshDeserialize;
use everlend_utils::{
    assert_account_key, assert_owned_by, assert_rent_exempt, assert_signer, assert_uninitialized,
    assert_zero_amount, cpi, find_program_address, EverlendError,
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

use crate::{
    find_pool_borrow_authority_program_address, find_pool_program_address,
    find_pool_withdraw_authority_program_address,
    instruction::CollateralPoolsInstruction,
    state::{
        InitPoolBorrowAuthorityParams, InitPoolMarketParams, InitPoolParams, Pool,
        PoolBorrowAuthority, PoolMarket, PoolWithdrawAuthority,
    },
    utils::*,
};

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

        // Check programs
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
        let token_mint_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        assert_owned_by(pool_market_info, program_id)?;

        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
        assert_account_key(manager_info, &pool_market.manager)?;

        let (pool_pubkey, bump_seed) =
            find_pool_program_address(program_id, pool_market_info.key, token_mint_info.key);
        assert_account_key(pool_info, &pool_pubkey)?;

        let signers_seeds = &[
            &pool_market_info.key.to_bytes()[..32],
            &token_mint_info.key.to_bytes()[..32],
            &[bump_seed],
        ];
        cpi::system::create_account::<Pool>(
            program_id,
            manager_info.clone(),
            pool_info.clone(),
            &[signers_seeds],
            rent,
        )?;

        let mut pool = Pool::unpack_unchecked(&pool_info.data.borrow())?;
        assert_uninitialized(&pool)?;

        cpi::spl_token::initialize_account(
            token_account_info.clone(),
            token_mint_info.clone(),
            pool_market_authority_info.clone(),
            rent_info.clone(),
        )?;

        pool.init(InitPoolParams {
            pool_market: *pool_market_info.key,
            token_mint: *token_mint_info.key,
            token_account: *token_account_info.key,
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

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;

        // Get pool market state
        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;

        // Check manager
        assert_account_key(manager_info, &pool_market.manager)?;

        let pool = Pool::unpack(&pool_info.data.borrow())?;

        // Check pool accounts
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
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(pool_borrow_authority_info, program_id)?;

        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;

        // Check manager
        assert_account_key(manager_info, &pool_market.manager)?;

        let pool = Pool::unpack(&pool_info.data.borrow())?;

        // Check pool accounts
        assert_account_key(pool_market_info, &pool.pool_market)?;

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;

        // Check pool borrow authority accounts
        assert_account_key(pool_info, &pool_borrow_authority.pool)?;

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
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let receiver_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(pool_borrow_authority_info, program_id)?;

        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;

        // Check manager
        assert_account_key(manager_info, &pool_market.manager)?;

        let pool = Pool::unpack(&pool_info.data.borrow())?;

        // Check pool accounts
        // Check pool borrow authority accounts
        assert_account_key(pool_market_info, &pool.pool_market)?;

        // Get pool borrow authority state to check initialized
        let pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;
        assert_account_key(pool_info, &pool_borrow_authority.pool)?;

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

    /// Process CreatePoolWithdrawAuthority instruction
    pub fn create_pool_withdraw_authority(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_withdraw_authority_info = next_account_info(account_info_iter)?;
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;

        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
        assert_account_key(manager_info, &pool_market.manager)?;

        let pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;

        let (pool_withdraw_authority_pubkey, bump_seed) =
            find_pool_withdraw_authority_program_address(
                program_id,
                pool_info.key,
                withdraw_authority_info.key,
            );
        assert_account_key(
            pool_withdraw_authority_info,
            &pool_withdraw_authority_pubkey,
        )?;

        let signers_seeds = &[
            &pool_info.key.to_bytes()[..32],
            &withdraw_authority_info.key.to_bytes()[..32],
            &[bump_seed],
        ];
        cpi::system::create_account::<PoolWithdrawAuthority>(
            program_id,
            manager_info.clone(),
            pool_withdraw_authority_info.clone(),
            &[signers_seeds],
            rent,
        )?;

        let mut pool_withdraw_authority =
            PoolWithdrawAuthority::unpack_unchecked(&pool_withdraw_authority_info.data.borrow())?;
        assert_uninitialized(&pool_withdraw_authority)?;

        pool_withdraw_authority.init(*pool_info.key, *withdraw_authority_info.key);
        PoolWithdrawAuthority::pack(
            pool_withdraw_authority,
            *pool_withdraw_authority_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    /// Process DeletePoolWithdrawAuthority instruction
    pub fn delete_pool_withdraw_authority(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_withdraw_authority_info = next_account_info(account_info_iter)?;
        let receiver_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(pool_withdraw_authority_info, program_id)?;

        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
        assert_account_key(manager_info, &pool_market.manager)?;

        let pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;

        let pool_withdraw_authority =
            PoolWithdrawAuthority::unpack(&pool_withdraw_authority_info.data.borrow())?;
        assert_account_key(pool_info, &pool_withdraw_authority.pool)?;

        let receiver_starting_lamports = receiver_info.lamports();
        let pool_withdraw_authority_lamports = pool_withdraw_authority_info.lamports();
        **pool_withdraw_authority_info.lamports.borrow_mut() = 0;
        **receiver_info.lamports.borrow_mut() = receiver_starting_lamports
            .checked_add(pool_withdraw_authority_lamports)
            .ok_or(EverlendError::MathOverflow)?;

        PoolWithdrawAuthority::pack(
            Default::default(),
            *pool_withdraw_authority_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    /// Process Deposit instruction
    pub fn deposit(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_zero_amount(amount)?;
        assert_signer(user_transfer_authority_info)?;

        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;

        let pool = Pool::unpack(&pool_info.data.borrow())?;

        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;

        cpi::spl_token::transfer(
            source_info.clone(),
            token_account_info.clone(),
            user_transfer_authority_info.clone(),
            amount,
            &[],
        )?;

        Ok(())
    }

    /// Process Withdraw instruction
    pub fn withdraw(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_withdraw_authority_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let withdraw_authority_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_zero_amount(amount)?;
        assert_signer(withdraw_authority_info)?;

        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(pool_withdraw_authority_info, program_id)?;

        let pool = Pool::unpack(&pool_info.data.borrow())?;

        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;

        let pool_withdraw_authority =
            PoolWithdrawAuthority::unpack(&pool_withdraw_authority_info.data.borrow())?;

        assert_account_key(pool_info, &pool_withdraw_authority.pool)?;
        assert_account_key(
            withdraw_authority_info,
            &pool_withdraw_authority.withdraw_authority,
        )?;
        let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
        let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];
        cpi::spl_token::transfer(
            token_account_info.clone(),
            destination_info.clone(),
            pool_market_authority_info.clone(),
            amount,
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

        assert_signer(borrow_authority_info)?;

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(pool_borrow_authority_info, program_id)?;

        let mut pool = Pool::unpack(&pool_info.data.borrow())?;

        // Check pool accounts
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;

        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;

        // Check pool borrow authority accounts
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

        // Check interest ?

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

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(pool_borrow_authority_info, program_id)?;

        let mut pool = Pool::unpack(&pool_info.data.borrow())?;

        // Check pool accounts
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;

        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;

        // Check pool borrow authority accounts
        assert_account_key(pool_info, &pool_borrow_authority.pool)?;

        pool_borrow_authority.repay(amount)?;
        pool.repay(amount)?;

        // Check interest ?

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

    /// Process UpdateManager instruction
    pub fn update_manager(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let new_manager_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;
        assert_signer(new_manager_info)?;

        assert_owned_by(pool_market_info, program_id)?;

        let mut pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
        assert_account_key(manager_info, &pool_market.manager)?;

        pool_market.manager = *new_manager_info.key;

        PoolMarket::pack(pool_market, *pool_market_info.data.borrow_mut())?;

        Ok(())
    }

    /// Instruction processing router
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = CollateralPoolsInstruction::try_from_slice(input)?;

        match instruction {
            CollateralPoolsInstruction::InitPoolMarket => {
                msg!("CollateralPoolsInstruction: InitPoolMarket");
                Self::init_pool_market(program_id, accounts)
            }

            CollateralPoolsInstruction::CreatePool => {
                msg!("CollateralPoolsInstruction: CreatePool");
                Self::create_pool(program_id, accounts)
            }

            CollateralPoolsInstruction::CreatePoolBorrowAuthority { share_allowed } => {
                msg!("CollateralPoolsInstruction: CreatePoolBorrowAuthority");
                Self::create_pool_borrow_authority(program_id, share_allowed, accounts)
            }

            CollateralPoolsInstruction::UpdatePoolBorrowAuthority { share_allowed } => {
                msg!("CollateralPoolsInstruction: UpdatePoolBorrowAuthority");
                Self::update_pool_borrow_authority(program_id, share_allowed, accounts)
            }

            CollateralPoolsInstruction::DeletePoolBorrowAuthority => {
                msg!("CollateralPoolsInstruction: DeletePoolBorrowAuthority");
                Self::delete_pool_borrow_authority(program_id, accounts)
            }

            CollateralPoolsInstruction::CreatePoolWithdrawAuthority => {
                msg!("CollateralPoolsInstruction: CreatePoolWithdrawAuthority");
                Self::create_pool_withdraw_authority(program_id, accounts)
            }

            CollateralPoolsInstruction::DeletePoolWithdrawAuthority => {
                msg!("CollateralPoolsInstruction: DeletePoolWithdrawAuthority");
                Self::delete_pool_withdraw_authority(program_id, accounts)
            }

            CollateralPoolsInstruction::Deposit { amount } => {
                msg!("CollateralPoolsInstruction: Deposit");
                Self::deposit(program_id, amount, accounts)
            }

            CollateralPoolsInstruction::Withdraw { amount } => {
                msg!("CollateralPoolsInstruction: Withdraw");
                Self::withdraw(program_id, amount, accounts)
            }

            CollateralPoolsInstruction::Borrow { amount } => {
                msg!("CollateralPoolsInstruction: Borrow");
                Self::borrow(program_id, amount, accounts)
            }

            CollateralPoolsInstruction::Repay {
                amount,
                interest_amount,
            } => {
                msg!("CollateralPoolsInstruction: Repay");
                Self::repay(program_id, amount, interest_amount, accounts)
            }

            CollateralPoolsInstruction::UpdateManager => {
                msg!("CollateralPoolsInstruction: UpdateManager");
                Self::update_manager(program_id, accounts)
            }
        }
    }
}
