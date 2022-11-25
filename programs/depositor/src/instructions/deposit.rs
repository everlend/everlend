use crate::{
    state::{Depositor, Rebalancing, RebalancingOperation},
    utils::{collateral_storage, deposit, money_market},
    InternalMiningPDA, RebalancingPDA, TransitPDA,
};
use everlend_registry::state::RegistryMarkets;
use everlend_utils::{assert_account_key, find_program_address, AccountLoader, EverlendError, PDA};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, sysvar::clock, sysvar::Sysvar,
};
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct DepositContext<'a, 'b> {
    registry: &'a AccountInfo<'b>,
    depositor: &'a AccountInfo<'b>,
    depositor_authority: &'a AccountInfo<'b>,
    rebalancing: &'a AccountInfo<'b>,

    collateral_transit: &'a AccountInfo<'b>,
    collateral_mint: &'a AccountInfo<'b>,

    liquidity_transit: &'a AccountInfo<'b>,
    liquidity_mint: &'a AccountInfo<'b>,

    clock: &'a AccountInfo<'b>,
    executor: &'a AccountInfo<'b>,
    internal_mining: &'a AccountInfo<'b>,

    money_market_program: &'a AccountInfo<'b>,
}

impl<'a, 'b> DepositContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<DepositContext<'a, 'b>, ProgramError> {
        let registry = AccountLoader::next_with_owner(account_info_iter, &everlend_registry::id())?;

        let depositor = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let depositor_authority = AccountLoader::next_unchecked(account_info_iter)?; //Signer PDA
        let rebalancing = AccountLoader::next_with_owner(account_info_iter, program_id)?;

        let liquidity_transit =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let liquidity_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let collateral_transit =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let collateral_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let executor = AccountLoader::next_signer(account_info_iter)?;

        let clock = AccountLoader::next_with_key(account_info_iter, &clock::id())?;

        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;

        let money_market_program = AccountLoader::next_unchecked(account_info_iter)?;
        let internal_mining = AccountLoader::next_optional(account_info_iter, program_id)?;

        Ok(DepositContext {
            registry,
            depositor,
            depositor_authority,
            rebalancing,
            collateral_transit,
            collateral_mint,
            liquidity_transit,
            liquidity_mint,
            internal_mining,
            executor,
            money_market_program,
            clock,
        })
    }

    /// Process instruction
    pub fn process(
        &self,
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> ProgramResult {
        {
            let depositor = Depositor::unpack(&self.depositor.data.borrow())?;

            assert_account_key(self.executor, &depositor.rebalance_executor)?;
            assert_account_key(self.registry, &depositor.registry)?;
        }

        {
            // Check rebalancing
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

        {
            // Check transit: liquidity
            let (liquidity_transit_pubkey, _) = TransitPDA {
                seed: "",
                depositor: *self.depositor.key,
                mint: *self.liquidity_mint.key,
            }
            .find_address(program_id);
            assert_account_key(self.liquidity_transit, &liquidity_transit_pubkey)?;
        }

        {
            // Check transit: collateral
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

        {
            let (internal_mining_pubkey, _) = InternalMiningPDA {
                liquidity_mint: *self.liquidity_mint.key,
                collateral_mint: *self.collateral_mint.key,
                depositor: *self.depositor.key,
            }
            .find_address(program_id);
            assert_account_key(self.internal_mining, &internal_mining_pubkey)?;
        }

        let registry_markets = RegistryMarkets::unpack_from_slice(&self.registry.data.borrow())?;

        // let money_market =
        let (money_market, is_mining) = money_market(
            &registry_markets,
            program_id,
            self.money_market_program,
            account_info_iter,
            self.internal_mining,
            self.collateral_mint.key,
            self.depositor_authority.key,
            self.depositor.key,
            self.liquidity_mint,
        )?;

        let collateral_stor = collateral_storage(
            &registry_markets,
            self.collateral_mint,
            self.depositor_authority,
            account_info_iter,
            false,
            is_mining,
        )?;

        {
            let step = rebalancing.next_step();

            if step.operation != RebalancingOperation::Deposit {
                return Err(EverlendError::InvalidRebalancingOperation.into());
            }

            if registry_markets.money_markets[usize::from(step.money_market_index)]
                != *self.money_market_program.key
            {
                return Err(EverlendError::InvalidRebalancingMoneyMarket.into());
            }
            msg!("Deposit");
            let collateral_amount = deposit(
                self.collateral_transit,
                self.collateral_mint,
                self.liquidity_transit,
                self.depositor_authority,
                self.clock,
                &money_market,
                is_mining,
                collateral_stor,
                step.liquidity_amount,
                &[signers_seeds],
            )?;

            let clock = Clock::from_account_info(self.clock)?;

            rebalancing.execute_step(
                RebalancingOperation::Deposit,
                Some(collateral_amount),
                clock.slot,
            )?;
        }

        Rebalancing::pack(rebalancing, *self.rebalancing.data.borrow_mut())?;

        Ok(())
    }
}
