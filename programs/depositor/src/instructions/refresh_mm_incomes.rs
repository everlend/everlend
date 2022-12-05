use crate::{
    state::{Depositor, Rebalancing, RebalancingOperation},
    // utils::{collateral_storage, deposit, money_market, withdraw},
    InternalMiningPDA, RebalancingPDA, TransitPDA,
};
use everlend_income_pools::utils::IncomePoolAccounts;
use everlend_registry::state::RegistryMarkets;
use everlend_utils::{assert_account_key, find_program_address, AccountLoader, EverlendError, PDA};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, sysvar::clock, sysvar::Sysvar,
};
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct RefreshMMIncomesContext<'a, 'b> {
    registry: &'a AccountInfo<'b>,
    depositor: &'a AccountInfo<'b>,
    depositor_authority: &'a AccountInfo<'b>,
    rebalancing: &'a AccountInfo<'b>,

    collateral_transit: &'a AccountInfo<'b>,
    collateral_mint: &'a AccountInfo<'b>,

    liquidity_transit: &'a AccountInfo<'b>,
    liquidity_reserve_transit: &'a AccountInfo<'b>,
    liquidity_mint: &'a AccountInfo<'b>,

    clock: &'a AccountInfo<'b>,
    executor: &'a AccountInfo<'b>,
    internal_mining: &'a AccountInfo<'b>,

    money_market_program: &'a AccountInfo<'b>,

    income_pool_accounts: IncomePoolAccounts<'a, 'b>,
}

impl<'a, 'b> RefreshMMIncomesContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<RefreshMMIncomesContext<'a, 'b>, ProgramError> {
        let registry = AccountLoader::next_with_owner(account_info_iter, &everlend_registry::id())?;

        let depositor = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let depositor_authority = AccountLoader::next_unchecked(account_info_iter)?; // Is PDA signer account of this program
        let rebalancing = AccountLoader::next_with_owner(account_info_iter, program_id)?;

        let income_pool_market =
            AccountLoader::next_with_owner(account_info_iter, &everlend_income_pools::id())?;
        let income_pool =
            AccountLoader::next_with_owner(account_info_iter, &everlend_income_pools::id())?;
        let income_pool_token_account =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let income_pool_accounts = IncomePoolAccounts {
            pool_market: income_pool_market,
            pool: income_pool,
            token_account: income_pool_token_account,
        };

        let collateral_transit =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let collateral_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let liquidity_transit =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let liquidity_reserve_transit =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let liquidity_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let executor = AccountLoader::next_signer(account_info_iter)?;

        let clock = AccountLoader::next_with_key(account_info_iter, &clock::id())?;

        let _token_program_info =
            AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;
        let _everlend_income_pools_info =
            AccountLoader::next_with_key(account_info_iter, &everlend_income_pools::id())?;

        let money_market_program = AccountLoader::next_unchecked(account_info_iter)?;

        // Optional account
        let internal_mining = AccountLoader::next_optional(account_info_iter, program_id)?;

        Ok(RefreshMMIncomesContext {
            registry,
            depositor,
            depositor_authority,
            rebalancing,
            collateral_transit,
            collateral_mint,
            liquidity_transit,
            liquidity_reserve_transit,
            liquidity_mint,
            internal_mining,
            executor,
            money_market_program,
            clock,
            income_pool_accounts,
        })
    }

    /// Process instruction
    pub fn process(
        &self,
        program_id: &Pubkey,
        account_info_iter: &'a mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> ProgramResult {
        {
            let depositor = Depositor::unpack(&self.depositor.data.borrow())?;
            assert_account_key(self.executor, &depositor.rebalance_executor)?;

            assert_account_key(self.registry, &depositor.registry)?;
        }

        let registry_markets = RegistryMarkets::unpack_from_slice(&self.registry.data.borrow())?;

        // Check rebalancing
        {
            let (rebalancing_pubkey, _) = RebalancingPDA {
                depositor: *self.depositor.key,
                mint: *self.liquidity_mint.key,
            }
            .find_address(program_id);
            assert_account_key(self.rebalancing, &rebalancing_pubkey)?;
        }

        let mut rebalancing = Rebalancing::unpack(&self.rebalancing.data.borrow())?;
        assert_account_key(self.depositor, &rebalancing.depositor)?;
        assert_account_key(self.liquidity_mint, &rebalancing.mint)?;

        if rebalancing.is_completed() {
            return Err(EverlendError::RebalancingIsCompleted.into());
        }

        // Check transit: liquidity
        {
            let (liquidity_transit_pubkey, _) = TransitPDA {
                seed: "",
                depositor: *self.depositor.key,
                mint: *self.liquidity_mint.key,
            }
            .find_address(program_id);
            assert_account_key(self.liquidity_transit, &liquidity_transit_pubkey)?;
        }

        // Check transit: liquidity reserve
        {
            let (liquidity_reserve_transit_pubkey, _) = TransitPDA {
                seed: "reserve",
                depositor: *self.depositor.key,
                mint: *self.liquidity_mint.key,
            }
            .find_address(program_id);
            assert_account_key(
                self.liquidity_reserve_transit,
                &liquidity_reserve_transit_pubkey,
            )?;
        }

        // Check transit: collateral
        {
            let (collateral_transit_pubkey, _) = TransitPDA {
                seed: "",
                depositor: *self.depositor.key,
                mint: *self.collateral_mint.key,
            }
            .find_address(program_id);
            assert_account_key(self.collateral_transit, &collateral_transit_pubkey)?;
        }

        let signers_seeds = {
            // Create depositor authority account
            let (depositor_authority_pubkey, bump_seed) =
                find_program_address(program_id, self.depositor.key);
            assert_account_key(self.depositor_authority, &depositor_authority_pubkey)?;
            &[&self.depositor.key.to_bytes()[..32], &[bump_seed]]
        };

        // Check internal mining account
        {
            let (internal_mining_pubkey, _) = InternalMiningPDA {
                liquidity_mint: *self.liquidity_mint.key,
                collateral_mint: *self.collateral_mint.key,
                depositor: *self.depositor.key,
            }
            .find_address(program_id);
            assert_account_key(self.internal_mining, &internal_mining_pubkey)?;
        }

        let clock = Clock::from_account_info(self.clock)?;

        // let (money_market, is_mining) = money_market(
        //     &registry_markets,
        //     program_id,
        //     self.money_market_program,
        //     account_info_iter,
        //     self.internal_mining,
        //     self.collateral_mint.key,
        //     self.depositor_authority.key,
        //     self.depositor.key,
        //     self.liquidity_mint,
        // )?;

        // let collateral_stor = collateral_storage(
        //     &registry_markets,
        //     self.collateral_mint,
        //     self.depositor_authority,
        //     account_info_iter,
        //     true,
        //     is_mining,
        // )?;

        // Check two step operation
        let (withdraw_step, deposit_step) = rebalancing.next_refresh_steps()?;

        if withdraw_step.money_market_index != deposit_step.money_market_index {
            return Err(EverlendError::InvalidRebalancingMoneyMarket.into());
        };

        // For both steps money_market is equal so check one of them
        if !registry_markets.money_markets[usize::from(withdraw_step.money_market_index)]
            .eq(self.money_market_program.key)
        {
            return Err(EverlendError::InvalidRebalancingMoneyMarket.into());
        }

        if withdraw_step.operation != RebalancingOperation::RefreshWithdraw
            || deposit_step.operation != RebalancingOperation::RefreshDeposit
        {
            return Err(EverlendError::InvalidRebalancingOperation.into());
        }

        // money_market.refresh_reserve(self.clock.clone())?;

        // Skip the refresh steps if the money market has no income and the amount of liquidity is the same for withdrawal and deposit
        // if !money_market.is_income(
        //     withdraw_step.collateral_amount.unwrap(),
        //     withdraw_step.liquidity_amount,
        // )? && withdraw_step.liquidity_amount == deposit_step.liquidity_amount
        // {
        //     msg!("Zero income amount. Skipping refresh step");
        //     rebalancing.skip_refresh_steps(clock.slot)?;
        //     Rebalancing::pack(rebalancing, *self.rebalancing.data.borrow_mut())?;
        //
        //     return Ok(());
        // }
        msg!("Refresh Withdraw");
        // withdraw(
        //     self.income_pool_accounts,
        //     self.collateral_transit,
        //     self.collateral_mint,
        //     self.liquidity_transit,
        //     self.liquidity_reserve_transit,
        //     self.depositor_authority,
        //     self.clock,
        //     &money_market,
        //     is_mining,
        //     &collateral_stor,
        //     withdraw_step.collateral_amount.unwrap(),
        //     withdraw_step.liquidity_amount,
        //     &[signers_seeds],
        // )?;

        rebalancing.execute_step(RebalancingOperation::RefreshWithdraw, None, clock.slot)?;

        // money_market.refresh_reserve(self.clock.clone())?;
        msg!("Refresh Deposit");
        // let collateral_amount = deposit(
        //     self.collateral_transit,
        //     self.collateral_mint,
        //     self.liquidity_transit,
        //     self.depositor_authority,
        //     self.clock,
        //     &money_market,
        //     is_mining,
        //     collateral_stor,
        //     deposit_step.liquidity_amount,
        //     &[signers_seeds],
        // )?;
let collateral_amount = 0;
        rebalancing.execute_step(
            RebalancingOperation::RefreshDeposit,
            Some(collateral_amount),
            clock.slot,
        )?;

        Rebalancing::pack(rebalancing, *self.rebalancing.data.borrow_mut())?;

        Ok(())
    }
}
