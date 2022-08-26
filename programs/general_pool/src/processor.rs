//! Program state processor
use borsh::BorshDeserialize;
use everlend_registry::state::Registry;
use everlend_utils::{
    assert_account_key, assert_owned_by, assert_rent_exempt, assert_signer, assert_uninitialized,
    cpi::{
        self,
        metaplex::{create_metadata, update_metadata},
        rewards::{deposit_mining, initialize_mining, withdraw_mining},
    },
    find_program_address, EverlendError,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_token::state::{Account, Mint};

use crate::{
    find_pool_borrow_authority_program_address, find_pool_program_address,
    find_transit_program_address, find_transit_sol_unwrap_address,
    find_withdrawal_request_program_address, find_withdrawal_requests_program_address,
    instruction::LiquidityPoolsInstruction,
    state::{
        InitPoolBorrowAuthorityParams, InitPoolMarketParams, InitPoolParams, Pool,
        PoolBorrowAuthority, PoolConfig, PoolMarket, WithdrawalRequest, WithdrawalRequests,
    },
    utils::*,
    withdrawal_requests_seed,
};
use crate::{
    find_pool_config_program_address,
    state::{InitWithdrawalRequestParams, InitWithdrawalRequestsParams, WITHDRAW_DELAY},
};

/// Program state handler.
pub struct Processor {}

impl Processor {
    /// Process InitPoolMarket instruction
    pub fn init_pool_market(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let registry_info = next_account_info(account_info_iter)?;
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
            registry: *registry_info.key,
        });

        PoolMarket::pack(pool_market, *pool_market_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process CreatePool instruction
    pub fn create_pool(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_config = next_account_info(account_info_iter)?;
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

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;

        // Get pool market state
        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;

        // Check manager
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
        let (transit_pubkey, transit_bump_seed) =
            find_transit_program_address(program_id, pool_market_info.key, pool_mint_info.key);
        assert_account_key(transit_info, &transit_pubkey)?;

        let transit_signers_seeds = &[
            br"transit",
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
        let (withdrawal_requests_pubkey, bump_seed) = find_withdrawal_requests_program_address(
            program_id,
            pool_market_info.key,
            token_mint_info.key,
        );
        assert_account_key(withdrawal_requests_info, &withdrawal_requests_pubkey)?;

        let withdrawal_requests_seed = withdrawal_requests_seed();
        let signers_seeds = &[
            withdrawal_requests_seed.as_bytes(),
            &pool_market_info.key.to_bytes()[..32],
            &token_mint_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        cpi::system::create_account::<WithdrawalRequests>(
            program_id,
            manager_info.clone(),
            withdrawal_requests_info.clone(),
            &[signers_seeds],
            rent,
        )?;

        let mut withdrawal_requests =
            WithdrawalRequests::unpack_unchecked(&withdrawal_requests_info.data.borrow())?;
        assert_uninitialized(&withdrawal_requests)?;

        withdrawal_requests.init(InitWithdrawalRequestsParams {
            pool: *pool_info.key,
            mint: *token_mint_info.key,
        });

        WithdrawalRequests::pack(
            withdrawal_requests,
            *withdrawal_requests_info.data.borrow_mut(),
        )?;

        pool.init(InitPoolParams {
            pool_market: *pool_market_info.key,
            token_mint: *token_mint_info.key,
            token_account: *token_account_info.key,
            pool_mint: *pool_mint_info.key,
        });

        Pool::pack(pool, *pool_info.data.borrow_mut())?;

        // TODO: Create Pool config
        // find_pool_config_program_address

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
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(pool_borrow_authority_info, program_id)?;

        // Get pool market state
        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
        assert_account_key(manager_info, &pool_market.manager)?;

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;
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

    /// Process Deposit instruction
    pub fn deposit(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_config_info = next_account_info(account_info_iter)?;
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        // mining accounts
        let mining_reward_pool = next_account_info(account_info_iter)?;
        let mining_reward_acc = next_account_info(account_info_iter)?;
        let everlend_config = next_account_info(account_info_iter)?;
        let everlend_rewards_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(everlend_config, &eld_config::id())?;
        assert_account_key(everlend_rewards_program_info, &eld_rewards::id())?;

        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(user_transfer_authority_info)?;
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;

        // Check pool accounts
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;
        assert_account_key(pool_mint_info, &pool.pool_mint)?;

        let (pool_config_pubkey, _) = find_pool_config_program_address(program_id, pool_info.key);
        assert_account_key(pool_config_info, &pool_config_pubkey)?;

        let pool_config = PoolConfig::unpack(&pool_config_info.data.borrow())?;
        if amount < pool_config.deposit_minimum {
            return Err(EverlendError::DepositAmountTooSmall.into());
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

        if mint_amount == 0 {
            return Err(EverlendError::DepositAmountTooSmall.into());
        }

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

        let (pool_pubkey, pool_bump_seed) =
            find_pool_program_address(program_id, &pool.pool_market, &pool.token_mint);
        assert_account_key(pool_info, &pool_pubkey)?;

        let pool_seeds: &[&[u8]] = &[
            &pool.pool_market.to_bytes()[..32],
            &pool.token_mint.to_bytes()[..32],
            &[pool_bump_seed],
        ];

        assert_owned_by(mining_reward_pool, &eld_rewards::id())?;
        assert_owned_by(mining_reward_acc, &eld_rewards::id())?;

        deposit_mining(
            everlend_rewards_program_info.key,
            everlend_config.clone(),
            mining_reward_pool.clone(),
            mining_reward_acc.clone(),
            user_transfer_authority_info.clone(),
            pool_info.to_owned(),
            mint_amount,
            &[pool_seeds],
        )?;

        Ok(())
    }

    /// Process Withdraw instruction
    pub fn withdraw(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let withdrawal_requests_info = next_account_info(account_info_iter)?;
        let withdrawal_request_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let collateral_transit_info = next_account_info(account_info_iter)?;
        let from_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(withdrawal_requests_info, program_id)?;
        assert_owned_by(withdrawal_request_info, program_id)?;

        // Check collateral token transit account
        let (collateral_transit_pubkey, _) =
            find_transit_program_address(program_id, pool_market_info.key, pool_mint_info.key);
        assert_account_key(collateral_transit_info, &collateral_transit_pubkey)?;

        // We don't check the pool pda, because it's created from the program
        // and is linked to the pool market

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;
        assert_account_key(pool_mint_info, &pool.pool_mint)?;

        // We don't check the withdrawal requests pda, because it's created from the program
        // and is linked to the pool

        let mut withdrawal_requests =
            WithdrawalRequests::unpack(&withdrawal_requests_info.data.borrow())?;
        assert_account_key(pool_info, &withdrawal_requests.pool)?;

        let withdrawal_request = WithdrawalRequest::unpack(&withdrawal_request_info.data.borrow())?;

        // Check withdraw request accounts
        assert_account_key(pool_info, &withdrawal_request.pool)?;
        assert_account_key(destination_info, &withdrawal_request.destination)?;
        assert_account_key(from_info, &withdrawal_request.from)?;

        // Check that enough time has passed to make a withdraw
        if withdrawal_request.ticket > clock.slot {
            return Err(EverlendError::WithdrawRequestsInvalidTicket.into());
        }

        let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
        let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

        // In the case of a SOL token, we do unwrap SPL token,
        // the destination can be any account

        if pool.token_mint == spl_token::native_mint::id() {
            let token_mint_info = next_account_info(account_info_iter)?;
            assert_account_key(token_mint_info, &pool.token_mint)?;

            let unwrap_sol_info = next_account_info(account_info_iter)?;

            // Check transit: unwrapped sol
            let (unwrap_sol_pubkey, bump_seed) =
                find_transit_sol_unwrap_address(program_id, withdrawal_request_info.key);
            assert_account_key(unwrap_sol_info, &unwrap_sol_pubkey)?;

            let signer_info = next_account_info(account_info_iter)?;

            let rent_info = next_account_info(account_info_iter)?;
            let rent = &Rent::from_account_info(rent_info)?;
            let _system_info = next_account_info(account_info_iter)?;

            let unwrap_acc_signers_seeds = &[
                br"unwrap",
                &withdrawal_request_info.key.to_bytes()[..32],
                &[bump_seed],
            ];

            cpi::system::create_account::<spl_token::state::Account>(
                &spl_token::id(),
                signer_info.clone(),
                unwrap_sol_info.clone(),
                &[unwrap_acc_signers_seeds],
                rent,
            )?;

            cpi::spl_token::initialize_account(
                unwrap_sol_info.clone(),
                token_mint_info.clone(),
                pool_market_authority_info.clone(),
                rent_info.clone(),
            )?;

            // Transfer from token account to destination
            cpi::spl_token::transfer(
                token_account_info.clone(),
                unwrap_sol_info.clone(),
                pool_market_authority_info.clone(),
                withdrawal_request.liquidity_amount,
                &[signers_seeds],
            )?;

            cpi::spl_token::close_account(
                signer_info.clone(),
                unwrap_sol_info.clone(),
                pool_market_authority_info.clone(),
                &[signers_seeds],
            )?;

            cpi::system::transfer(
                signer_info.clone(),
                destination_info.clone(),
                withdrawal_request.liquidity_amount,
                &[],
            )?;
        } else {
            // Transfer from token account to destination
            cpi::spl_token::transfer(
                token_account_info.clone(),
                destination_info.clone(),
                pool_market_authority_info.clone(),
                withdrawal_request.liquidity_amount,
                &[signers_seeds],
            )?;
        };

        // Burn from transit collateral pool token
        cpi::spl_token::burn(
            pool_mint_info.clone(),
            collateral_transit_info.clone(),
            pool_market_authority_info.clone(),
            withdrawal_request.collateral_amount,
            &[signers_seeds],
        )?;

        withdrawal_requests.process(withdrawal_request.liquidity_amount)?;

        // Close withdraw account and return rent
        let from_starting_lamports = from_info.lamports();
        let withdraw_request_lamports = withdrawal_request_info.lamports();

        **withdrawal_request_info.lamports.borrow_mut() = 0;
        **from_info.lamports.borrow_mut() = from_starting_lamports
            .checked_add(withdraw_request_lamports)
            .ok_or(EverlendError::MathOverflow)?;

        WithdrawalRequests::pack(
            withdrawal_requests,
            *withdrawal_requests_info.data.borrow_mut(),
        )?;
        WithdrawalRequest::pack(
            Default::default(),
            *withdrawal_request_info.data.borrow_mut(),
        )?;

        Ok(())
    }

    /// Process Withdraw request instruction
    pub fn withdraw_request(
        program_id: &Pubkey,
        collateral_amount: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_config_info = next_account_info(account_info_iter)?;
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let withdrawal_requests_info = next_account_info(account_info_iter)?;
        let withdrawal_request_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_account_info = next_account_info(account_info_iter)?;
        let collateral_transit_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        // mining accounts
        let mining_reward_pool = next_account_info(account_info_iter)?;
        let mining_reward_acc = next_account_info(account_info_iter)?;
        let everlend_config = next_account_info(account_info_iter)?;
        let everlend_rewards_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(everlend_config, &eld_config::id())?;
        assert_account_key(everlend_rewards_program_info, &eld_rewards::id())?;

        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(user_transfer_authority_info)?;

        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(withdrawal_requests_info, program_id)?;

        let pool = Pool::unpack(&pool_info.data.borrow())?;

        // Check pool accounts
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;
        assert_account_key(pool_mint_info, &pool.pool_mint)?;

        // In all cases except SOL token, we must check destination account
        if pool.token_mint != spl_token::native_mint::id() {
            let destination_account = Account::unpack(&destination_info.data.borrow())?;
            if pool.token_mint != destination_account.mint {
                return Err(ProgramError::InvalidArgument);
            }
        }

        // Check transit: collateral
        let (collateral_transit_pubkey, _) =
            find_transit_program_address(program_id, pool_market_info.key, pool_mint_info.key);
        assert_account_key(collateral_transit_info, &collateral_transit_pubkey)?;

        let mut withdrawal_requests =
            WithdrawalRequests::unpack(&withdrawal_requests_info.data.borrow())?;

        // Check withdrawal requests accounts
        assert_account_key(pool_info, &withdrawal_requests.pool)?;

        // Check withdrawal request
        let (withdrawal_request_pubkey, bump_seed) = find_withdrawal_request_program_address(
            program_id,
            withdrawal_requests_info.key,
            user_transfer_authority_info.key,
        );
        assert_account_key(withdrawal_request_info, &withdrawal_request_pubkey)?;

        let total_incoming =
            total_pool_amount(token_account_info.clone(), pool.total_amount_borrowed)?;
        let total_minted = Mint::unpack_unchecked(&pool_mint_info.data.borrow())?.supply;

        let liquidity_amount = (collateral_amount as u128)
            .checked_mul(total_incoming as u128)
            .ok_or(EverlendError::MathOverflow)?
            .checked_div(total_minted as u128)
            .ok_or(EverlendError::MathOverflow)? as u64;

        let (pool_config_pubkey, _) = find_pool_config_program_address(program_id, pool_info.key);
        assert_account_key(pool_config_info, &pool_config_pubkey)?;

        let pool_config = PoolConfig::unpack(&pool_config_info.data.borrow())?;
        if liquidity_amount < pool_config.withdraw_minimum {
            return Err(EverlendError::WithdrawAmountTooSmall.into());
        }

        // Transfer
        cpi::spl_token::transfer(
            source_info.clone(),
            collateral_transit_info.clone(),
            user_transfer_authority_info.clone(),
            collateral_amount,
            &[],
        )?;

        let signers_seeds = &[
            br"withdrawal",
            &withdrawal_requests_info.key.to_bytes()[..32],
            &user_transfer_authority_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        cpi::system::create_account::<WithdrawalRequest>(
            program_id,
            user_transfer_authority_info.clone(),
            withdrawal_request_info.clone(),
            &[signers_seeds],
            rent,
        )?;

        let mut withdrawal_request =
            WithdrawalRequest::unpack_unchecked(&withdrawal_request_info.data.borrow())?;

        withdrawal_request.init(InitWithdrawalRequestParams {
            pool: *pool_info.key,
            from: *user_transfer_authority_info.key,
            source: *source_info.key,
            destination: *destination_info.key,
            liquidity_amount,
            collateral_amount,
            ticket: clock.slot + WITHDRAW_DELAY,
        });

        withdrawal_requests.add(liquidity_amount)?;

        WithdrawalRequests::pack(
            withdrawal_requests,
            *withdrawal_requests_info.data.borrow_mut(),
        )?;
        WithdrawalRequest::pack(
            withdrawal_request,
            *withdrawal_request_info.data.borrow_mut(),
        )?;

        // Mining reward
        let (pool_pubkey, pool_bump_seed) =
            find_pool_program_address(program_id, &pool.pool_market, &pool.token_mint);
        assert_account_key(pool_info, &pool_pubkey)?;

        let pool_seeds: &[&[u8]] = &[
            &pool.pool_market.to_bytes()[..32],
            &pool.token_mint.to_bytes()[..32],
            &[pool_bump_seed],
        ];

        assert_owned_by(mining_reward_pool, &eld_rewards::id())?;
        assert_owned_by(mining_reward_acc, &eld_rewards::id())?;

        withdraw_mining(
            everlend_rewards_program_info.key,
            everlend_config.clone(),
            mining_reward_pool.clone(),
            mining_reward_acc.clone(),
            user_transfer_authority_info.clone(),
            pool_info.to_owned(),
            collateral_amount,
            &[pool_seeds],
        )?;

        Ok(())
    }

    /// Process Cancel withdraw request instruction
    pub fn cancel_withdraw_request(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let withdrawal_requests_info = next_account_info(account_info_iter)?;
        let withdrawal_request_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let collateral_transit_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let from_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;
        assert_owned_by(withdrawal_requests_info, program_id)?;
        assert_owned_by(withdrawal_request_info, program_id)?;

        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;

        // Check manager
        assert_account_key(manager_info, &pool_market.manager)?;

        // Check collateral token transit account
        let (collateral_transit_pubkey, _) =
            find_transit_program_address(program_id, pool_market_info.key, pool_mint_info.key);
        assert_account_key(collateral_transit_info, &collateral_transit_pubkey)?;

        // We don't check the pool pda, because it's created from the program
        // and is linked to the pool market

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(pool_mint_info, &pool.pool_mint)?;

        // We don't check the withdrawal requests pda, because it's created from the program
        // and is linked to the pool

        let mut withdrawal_requests =
            WithdrawalRequests::unpack(&withdrawal_requests_info.data.borrow())?;

        // Check withdrawal requests accounts
        assert_account_key(pool_info, &withdrawal_requests.pool)?;

        let withdrawal_request = WithdrawalRequest::unpack(&withdrawal_request_info.data.borrow())?;

        // Check withdrawal request accounts
        assert_account_key(pool_info, &withdrawal_request.pool)?;
        assert_account_key(source_info, &withdrawal_request.source)?;
        assert_account_key(from_info, &withdrawal_request.from)?;

        let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
        let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

        cpi::spl_token::transfer(
            collateral_transit_info.clone(),
            source_info.clone(),
            pool_market_authority_info.clone(),
            withdrawal_request.collateral_amount,
            &[signers_seeds],
        )?;

        withdrawal_requests.process(withdrawal_request.liquidity_amount)?;

        // Close withdraw account and return rent
        let from_starting_lamports = from_info.lamports();
        let withdraw_request_lamports = withdrawal_request_info.lamports();

        **withdrawal_request_info.lamports.borrow_mut() = 0;
        **from_info.lamports.borrow_mut() = from_starting_lamports
            .checked_add(withdraw_request_lamports)
            .ok_or(EverlendError::MathOverflow)?;

        WithdrawalRequests::pack(
            withdrawal_requests,
            *withdrawal_requests_info.data.borrow_mut(),
        )?;
        WithdrawalRequest::pack(
            withdrawal_request,
            *withdrawal_request_info.data.borrow_mut(),
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

        // Get pool state
        let mut pool = Pool::unpack(&pool_info.data.borrow())?;

        // Check pool accounts
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(token_account_info, &pool.token_account)?;

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;
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

    /// Migrate withdraw request
    pub fn init_user_mining(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let user_collateral_token_account_info = next_account_info(account_info_iter)?;
        let user_authority = next_account_info(account_info_iter)?;
        let registry_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let mining_reward_pool = next_account_info(account_info_iter)?;
        let mining_reward_acc = next_account_info(account_info_iter)?;

        let everlend_config = next_account_info(account_info_iter)?;
        let everlend_rewards_program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;

        assert_signer(manager_info)?;
        assert_owned_by(registry_info, &everlend_registry::id())?;

        let registry = Registry::unpack(&registry_info.data.borrow())?;
        assert_account_key(manager_info, &registry.manager)?;

        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;

        assert_owned_by(everlend_config, &eld_config::id())?;
        assert_owned_by(mining_reward_pool, &eld_rewards::id())?;
        assert_account_key(everlend_rewards_program_info, &eld_rewards::id())?;

        let pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;

        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
        assert_account_key(registry_info, &pool_market.registry)?;

        let (pool_pubkey, pool_bump_seed) =
            find_pool_program_address(program_id, &pool.pool_market, &pool.token_mint);
        assert_account_key(pool_info, &pool_pubkey)?;

        let pool_seeds: &[&[u8]] = &[
            &pool.pool_market.to_bytes()[..32],
            &pool.token_mint.to_bytes()[..32],
            &[pool_bump_seed],
        ];

        let user_account = Account::unpack(&user_collateral_token_account_info.data.borrow())?;
        if pool.pool_mint != user_account.mint {
            return Err(ProgramError::InvalidArgument);
        }

        // check authority
        if !user_account.owner.eq(user_authority.key) {
            return Err(ProgramError::InvalidArgument);
        }

        if !mining_reward_acc.owner.eq(&Pubkey::default()) {
            return Err(ProgramError::InvalidArgument);
        }

        initialize_mining(
            everlend_rewards_program_info.key,
            everlend_config.clone(),
            mining_reward_pool.clone(),
            mining_reward_acc.clone(),
            user_authority.clone(),
            manager_info.clone(),
            system_program_info.clone(),
            rent_info.clone(),
        )?;

        deposit_mining(
            everlend_rewards_program_info.key,
            everlend_config.clone(),
            mining_reward_pool.clone(),
            mining_reward_acc.clone(),
            user_authority.clone(),
            pool_info.to_owned(),
            user_account.amount,
            &[pool_seeds],
        )?;

        Ok(())
    }

    /// Migrate withdraw request
    pub fn migrate_instruction(_program_id: &Pubkey, _accounts: &[AccountInfo]) -> ProgramResult {
        Err(EverlendError::TemporaryUnavailable.into())
    }

    /// Migrate pool market
    pub fn close_pool_market(_program_id: &Pubkey, _accounts: &[AccountInfo]) -> ProgramResult {
        Err(EverlendError::TemporaryUnavailable.into())
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

    /// Process UpdateManager instruction
    pub fn set_token_metadata(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        name: String,
        symbol: String,
        uri: String,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let metadata_info = next_account_info(account_info_iter)?;
        let manager_info = next_account_info(account_info_iter)?;
        let metaplex_program_info = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;

        // Check manager
        assert_signer(manager_info)?;

        // Check programs
        assert_owned_by(pool_market_info, program_id)?;
        assert_owned_by(pool_info, program_id)?;

        // Get pool market state
        let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
        assert_account_key(manager_info, &pool_market.manager)?;

        // Get pool state
        let pool = Pool::unpack(&pool_info.data.borrow())?;
        assert_account_key(pool_market_info, &pool.pool_market)?;
        assert_account_key(pool_mint_info, &pool.pool_mint)?;

        // Get authority
        let (pool_market_authority, bump_seed) =
            find_program_address(program_id, pool_market_info.key);
        assert_account_key(pool_market_authority_info, &pool_market_authority)?;

        let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

        if metadata_info.owner.eq(&Pubkey::default()) {
            create_metadata(
                metaplex_program_info.clone(),
                metadata_info.clone(),
                pool_mint_info.clone(),
                pool_market_authority_info.clone(),
                manager_info.clone(),
                system_program_info.clone(),
                rent_info.clone(),
                name,
                symbol,
                uri,
                &[signers_seeds],
            )?;
        } else {
            update_metadata(
                metaplex_program_info.clone(),
                metadata_info.clone(),
                pool_mint_info.clone(),
                pool_market_authority_info.clone(),
                name,
                symbol,
                uri,
                &[signers_seeds],
            )?;
        }

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

            LiquidityPoolsInstruction::Withdraw => {
                msg!("LiquidityPoolsInstruction: Withdraw");
                Self::withdraw(program_id, accounts)
            }

            LiquidityPoolsInstruction::WithdrawRequest { collateral_amount } => {
                msg!("LiquidityPoolsInstruction: WithdrawRequest");
                Self::withdraw_request(program_id, collateral_amount, accounts)
            }

            LiquidityPoolsInstruction::CancelWithdrawRequest => {
                msg!("LiquidityPoolsInstruction: CancelWithdrawRequest");
                Self::cancel_withdraw_request(program_id, accounts)
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

            LiquidityPoolsInstruction::ClosePoolMarket => {
                msg!("LiquidityPoolsInstruction: ClosePoolMarket");
                Self::close_pool_market(program_id, accounts)
            }

            LiquidityPoolsInstruction::MigrationInstruction => {
                msg!("LiquidityPoolsInstruction: MigrationInstruction");
                Self::migrate_instruction(program_id, accounts)
            }

            LiquidityPoolsInstruction::InitUserMining => {
                msg!("LiquidityPoolsInstruction: InitUserMining");
                Self::init_user_mining(program_id, accounts)
            }

            LiquidityPoolsInstruction::UpdateManager => {
                msg!("LiquidityPoolsInstruction: UpdateManager");
                Self::update_manager(program_id, accounts)
            }

            LiquidityPoolsInstruction::SetTokenMetadata { name, symbol, uri } => {
                msg!("LiquidityPoolsInstruction: SetTokenMetadata");
                Self::set_token_metadata(program_id, accounts, name, symbol, uri)
            }
        }
    }
}
