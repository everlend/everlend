use everlend_utils::EverlendError;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey, program_pack::Pack, msg,
};
use everlend_registry::state::Registry;
use everlend_utils::{assert_account_key, find_program_address, AccountLoader};
use crate::state::{Rebalancing, Depositor};
use std::{iter::Enumerate, slice::Iter};
use spl_token::state::Account;

/// Instruction context
pub struct MigrateDepositorContext<'a, 'b> {
    rebalancing: &'a AccountInfo<'b>,
    depositor: &'a AccountInfo<'b>,
    depositor_authority: &'a AccountInfo<'b>,
    registry: &'a AccountInfo<'b>,
    liquidity_transit: &'a AccountInfo<'b>,

    general_pool_market: &'a AccountInfo<'b>,
    general_pool_market_authority: &'a AccountInfo<'b>,
    general_pool: &'a AccountInfo<'b>,
    general_pool_token_account: &'a AccountInfo<'b>,
    general_pool_borrow_authority: &'a AccountInfo<'b>,

    manager: &'a AccountInfo<'b>,
}

impl<'a, 'b> MigrateDepositorContext<'a, 'b> {
    /// New MigrateDepositor instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<MigrateDepositorContext<'a, 'b>, ProgramError> {
        let registry = AccountLoader::next_with_owner(account_info_iter, &everlend_registry::id())?;

        let depositor = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let depositor_authority = AccountLoader::next_unchecked(account_info_iter)?; //Signer PDA

        let rebalancing = AccountLoader::next_optional(account_info_iter, program_id)?;

        let general_pool_market =
            AccountLoader::next_with_owner(account_info_iter, &everlend_general_pool::id())?;
        let general_pool_market_authority = AccountLoader::next_unchecked(account_info_iter)?; //PDA signer
        let general_pool =
            AccountLoader::next_with_owner(account_info_iter, &everlend_general_pool::id())?;
        let general_pool_token_account =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let general_pool_borrow_authority = AccountLoader::next_unchecked(account_info_iter)?;

        let liquidity_transit =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let manager = AccountLoader::next_signer(account_info_iter)?;

       Ok(
           MigrateDepositorContext{
               rebalancing,
               depositor,
               depositor_authority,
               registry,
               liquidity_transit,

               general_pool_market,
               general_pool_market_authority,
               general_pool,
               general_pool_token_account,
               general_pool_borrow_authority,

               manager,
           }
       )
    }

    /// Process MigrateDepositor instruction
    pub fn process(
        &self,
        program_id: &Pubkey,
        _account_info_iter: &'a mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> ProgramResult {

        // Check manager
        {
            let depositor = Depositor::unpack(&self.depositor.data.borrow())?;
            assert_account_key(self.registry, &depositor.registry)?;
            let registry = Registry::unpack(&self.registry.data.borrow())?;
            assert_account_key(self.manager, &registry.manager)?;
        }


        let mut rebalancing = Rebalancing::unpack(&self.rebalancing.data.borrow())?;
        assert_account_key(self.depositor, &rebalancing.depositor)?;

        if rebalancing.total_distributed_liquidity()? != 0{
            msg!("Rebalance have distributed liquidity");
            return Err(EverlendError::RebalanceLiquidityCheckFailed.into())
        };

        let liquidity_transit = Account::unpack(&self.liquidity_transit.data.borrow())?;
        if !rebalancing.is_completed() || liquidity_transit.amount != rebalancing.amount_to_distribute{
            msg!("liquidity_transit.amount != rebalancing.amount_to_distribute");
            return Err(EverlendError::IncompleteRebalancing.into())
        }

        // Check depositor authority account
        let signers_seeds = {
            let (depositor_authority_pubkey, bump_seed) =
                find_program_address(program_id, self.depositor.key);
            assert_account_key(self.depositor_authority, &depositor_authority_pubkey)?;
            &[&self.depositor.key.to_bytes()[..32], &[bump_seed]]
        };

        msg!("Repay to General Pool");
        everlend_general_pool::cpi::repay(
            self.general_pool_market.clone(),
            self.general_pool_market_authority.clone(),
            self.general_pool.clone(),
            self.general_pool_borrow_authority.clone(),
            self.liquidity_transit.clone(),
            self.general_pool_token_account.clone(),
            self.depositor_authority.clone(),
            rebalancing.amount_to_distribute,
            0,
            &[signers_seeds],
        )?;

        // Flush depositor amount
        rebalancing.amount_to_distribute = 0;
        rebalancing.steps = vec![];
        Rebalancing::pack(rebalancing, *self.rebalancing.data.borrow_mut())?;

        Ok(())
    }
}
