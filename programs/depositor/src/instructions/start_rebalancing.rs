use crate::{
    find_rebalancing_program_address, find_transit_program_address,
    state::{Depositor, InitRebalancingParams, Rebalancing},
    utils::calculate_amount_to_distribute,
};
use everlend_general_pool::{find_withdrawal_requests_program_address, state::WithdrawalRequests};

use everlend_liquidity_oracle::{find_token_oracle_program_address, state::TokenOracle};
use everlend_registry::state::{Registry, RegistryMarkets};
use everlend_utils::{assert_account_key, cpi, find_program_address, AccountLoader, EverlendError};
use num_traits::Zero;
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, rent::Rent, system_program,
    sysvar::clock, sysvar::Sysvar, sysvar::SysvarId,
};
use spl_token::state::Account;
use std::cmp::min;
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct StartRebalancingContext<'a, 'b> {
    registry: &'a AccountInfo<'b>,
    depositor: &'a AccountInfo<'b>,
    depositor_authority: &'a AccountInfo<'b>,
    rebalancing: &'a AccountInfo<'b>,
    mint: &'a AccountInfo<'b>,
    general_pool_market: &'a AccountInfo<'b>,
    general_pool_market_authority: &'a AccountInfo<'b>,
    general_pool: &'a AccountInfo<'b>,
    general_pool_token_account: &'a AccountInfo<'b>,
    general_pool_borrow_authority: &'a AccountInfo<'b>,
    withdrawal_requests: &'a AccountInfo<'b>,
    liquidity_transit: &'a AccountInfo<'b>,
    liquidity_oracle: &'a AccountInfo<'b>,
    token_oracle: &'a AccountInfo<'b>,
    executor: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
    clock: &'a AccountInfo<'b>,
}

impl<'a, 'b> StartRebalancingContext<'a, 'b> {
    /// New StartRebalancing instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<StartRebalancingContext<'a, 'b>, ProgramError> {
        let registry = AccountLoader::next_with_owner(account_info_iter, &everlend_registry::id())?;

        let depositor = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let depositor_authority = AccountLoader::next_unchecked(account_info_iter)?; //Signer PDA

        let rebalancing = AccountLoader::next_optional(account_info_iter, program_id)?;
        let mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let general_pool_market =
            AccountLoader::next_with_owner(account_info_iter, &everlend_general_pool::id())?;
        let general_pool_market_authority = AccountLoader::next_unchecked(account_info_iter)?; //PDA signer
        let general_pool =
            AccountLoader::next_with_owner(account_info_iter, &everlend_general_pool::id())?;
        let general_pool_token_account =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let general_pool_borrow_authority = AccountLoader::next_unchecked(account_info_iter)?;
        let withdrawal_requests =
            AccountLoader::next_with_owner(account_info_iter, &everlend_general_pool::id())?;

        let liquidity_transit =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        // TODO: we can do it optional for refresh income case in the future
        let liquidity_oracle =
            AccountLoader::next_with_owner(account_info_iter, &everlend_liquidity_oracle::id())?;
        let token_oracle =
            AccountLoader::next_with_owner(account_info_iter, &everlend_liquidity_oracle::id())?;
        let executor = AccountLoader::next_signer(account_info_iter)?;

        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;
        let clock = AccountLoader::next_with_key(account_info_iter, &clock::id())?;

        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;
        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;
        let _general_pool_program =
            AccountLoader::next_with_key(account_info_iter, &everlend_general_pool::id())?;

        Ok(StartRebalancingContext {
            registry,
            depositor,
            depositor_authority,
            rebalancing,
            mint,
            general_pool_market,
            general_pool_market_authority,
            general_pool,
            general_pool_token_account,
            general_pool_borrow_authority,
            withdrawal_requests,
            liquidity_transit,
            liquidity_oracle,
            token_oracle,
            executor,
            rent,
            clock,
        })
    }

    /// Process StartRebalancing instruction
    pub fn process(
        &self,
        program_id: &Pubkey,
        _account_info_iter: &'a mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        refresh_income: bool,
    ) -> ProgramResult {
        {
            // Get depositor state
            let depositor = Depositor::unpack(&self.depositor.data.borrow())?;
            assert_account_key(self.executor, &depositor.rebalance_executor)?;
            assert_account_key(self.registry, &depositor.registry)?;
        }

        let registry = Registry::unpack(&self.registry.data.borrow())?;
        // Check root accounts
        assert_account_key(self.general_pool_market, &registry.general_pool_market)?;
        assert_account_key(self.liquidity_oracle, &registry.liquidity_oracle)?;

        let bump_seed = {
            // Check rebalancing
            let (rebalancing_pubkey, bump_seed) =
                find_rebalancing_program_address(program_id, self.depositor.key, self.mint.key);
            assert_account_key(self.rebalancing, &rebalancing_pubkey)?;
            bump_seed
        };

        // Create or get rebalancing account
        let mut rebalancing = match self.rebalancing.lamports() {
            // Create rebalancing account
            0 => {
                let signers_seeds = &[
                    "rebalancing".as_bytes(),
                    &self.depositor.key.to_bytes()[..32],
                    &self.mint.key.to_bytes()[..32],
                    &[bump_seed],
                ];

                let rent = &Rent::from_account_info(self.rent)?;

                cpi::system::create_account::<Rebalancing>(
                    program_id,
                    self.executor.clone(),
                    self.rebalancing.clone(),
                    &[signers_seeds],
                    rent,
                )?;

                let mut rebalancing =
                    Rebalancing::unpack_unchecked(&self.rebalancing.data.borrow())?;
                rebalancing.init(InitRebalancingParams {
                    depositor: *self.depositor.key,
                    mint: *self.mint.key,
                });

                rebalancing
            }
            _ => {
                let rebalancing = Rebalancing::unpack(&self.rebalancing.data.borrow())?;
                assert_account_key(self.depositor, &rebalancing.depositor)?;
                assert_account_key(self.mint, &rebalancing.mint)?;

                rebalancing
            }
        };

        // Check rebalancing is completed
        if !rebalancing.is_completed() {
            return Err(EverlendError::IncompleteRebalancing.into());
        }

        let general_pool_state =
            everlend_general_pool::state::Pool::unpack(&self.general_pool.data.borrow())?;

        {
            // Check token distribution
            let (token_oracle_pubkey, _) = find_token_oracle_program_address(
                &everlend_liquidity_oracle::id(),
                self.liquidity_oracle.key,
                self.mint.key,
            );
            assert_account_key(self.token_oracle, &token_oracle_pubkey)?;
        }

        {
            // Check general pool
            let (general_pool_pubkey, _) = everlend_general_pool::find_pool_program_address(
                &everlend_general_pool::id(),
                self.general_pool_market.key,
                self.mint.key,
            );
            assert_account_key(self.general_pool, &general_pool_pubkey)?;
        }

        {
            // Check general pool accounts
            assert_account_key(self.general_pool_market, &general_pool_state.pool_market)?;
            assert_account_key(
                self.general_pool_token_account,
                &general_pool_state.token_account,
            )?;
            assert_account_key(self.mint, &general_pool_state.token_mint)?;

            // Check withdrawal requests
            let (withdrawal_requests_pubkey, _) = find_withdrawal_requests_program_address(
                &everlend_general_pool::id(),
                self.general_pool_market.key,
                &general_pool_state.token_mint,
            );
            assert_account_key(self.withdrawal_requests, &withdrawal_requests_pubkey)?;
        }

        {
            // Check transit: liquidity
            let (liquidity_transit_pubkey, _) =
                find_transit_program_address(program_id, self.depositor.key, self.mint.key, "");
            assert_account_key(self.liquidity_transit, &liquidity_transit_pubkey)?;
        }

        let general_pool = Account::unpack(&self.general_pool_token_account.data.borrow())?;
        let liquidity_transit = Account::unpack(&self.liquidity_transit.data.borrow())?;
        let withdrawal_requests =
            WithdrawalRequests::unpack(&self.withdrawal_requests.data.borrow())?;

        let (available_liquidity, amount_to_distribute) = calculate_amount_to_distribute(
            rebalancing.total_distributed_liquidity(),
            liquidity_transit.amount,
            general_pool.amount,
            withdrawal_requests.liquidity_supply,
        )?;

        msg!(
            "available_liquidity: {} amount_to_distribute: {}",
            available_liquidity,
            amount_to_distribute
        );

        // Additional check for maths
        if available_liquidity != general_pool_state.total_amount_borrowed {
            return Err(EverlendError::RebalanceLiquidityCheckFailed.into());
        }

        let (depositor_authority_pubkey, bump_seed) =
            find_program_address(program_id, self.depositor.key);
        assert_account_key(self.depositor_authority, &depositor_authority_pubkey)?;
        let signers_seeds = &[&self.depositor.key.to_bytes()[..32], &[bump_seed]];

        if amount_to_distribute.gt(&available_liquidity) {
            let borrow_amount = amount_to_distribute
                .checked_sub(available_liquidity)
                .ok_or(EverlendError::MathOverflow)?;

            msg!("Borrow from General Pool");
            everlend_general_pool::cpi::borrow(
                self.general_pool_market.clone(),
                self.general_pool_market_authority.clone(),
                self.general_pool.clone(),
                self.general_pool_borrow_authority.clone(),
                self.liquidity_transit.clone(),
                self.general_pool_token_account.clone(),
                self.depositor_authority.clone(),
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
                    self.general_pool_market.clone(),
                    self.general_pool_market_authority.clone(),
                    self.general_pool.clone(),
                    self.general_pool_borrow_authority.clone(),
                    self.liquidity_transit.clone(),
                    self.general_pool_token_account.clone(),
                    self.depositor_authority.clone(),
                    repay_amount,
                    0,
                    &[signers_seeds],
                )?;
            }
        }

        let clock = Clock::from_account_info(self.clock)?;

        let markets_indexes = {
            let markets = RegistryMarkets::unpack_money_markets(&self.registry.data.borrow())?;

            markets
                .iter()
                .enumerate()
                .filter_map(|(index, mm)| {
                    if mm != &Default::default() {
                        return Some(index);
                    }
                    None
                })
                .collect()
        };

        // Compute rebalancing steps
        msg!("Computing");
        if refresh_income {
            rebalancing.compute_with_refresh_income(
                markets_indexes,
                registry.refresh_income_interval,
                clock.slot,
                amount_to_distribute,
            )?;
        } else {
            // Compute rebalancing steps
            let token_oracle = TokenOracle::unpack(&self.token_oracle.data.borrow())?;

            rebalancing.compute(
                markets_indexes,
                token_oracle,
                amount_to_distribute,
                clock.slot,
            )?;
        }

        Rebalancing::pack(rebalancing, *self.rebalancing.data.borrow_mut())?;

        Ok(())
    }
}
