use crate::{
    find_rebalancing_program_address,
    state::{Depositor, Rebalancing},
};
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_registry::state::Registry;
use everlend_utils::{assert_account_key, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey, system_program,
};
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct SetRebalancingContext<'a, 'b> {
    registry: &'a AccountInfo<'b>,
    depositor: &'a AccountInfo<'b>,
    rebalancing: &'a AccountInfo<'b>,
    liquidity_mint: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
}

impl<'a, 'b> SetRebalancingContext<'a, 'b> {
    /// New ResetRebalancing instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<SetRebalancingContext<'a, 'b>, ProgramError> {
        let registry = AccountLoader::next_with_owner(account_info_iter, &everlend_registry::id())?;
        let depositor = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let rebalancing = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let liquidity_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let manager = AccountLoader::next_signer(account_info_iter)?;

        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        Ok(SetRebalancingContext {
            registry,
            depositor,
            rebalancing,
            liquidity_mint,
            manager,
        })
    }

    /// Process ResetRebalancing instruction
    pub fn process(
        &self,
        program_id: &Pubkey,
        _account_info_iter: &'a mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        amount_to_distribute: u64,
        distributed_liquidity: u64,
        distribution_array: DistributionArray,
    ) -> ProgramResult {
        {
            // Get depositor state
            let depositor = Depositor::unpack(&self.depositor.data.borrow())?;
            // Check registry
            assert_account_key(self.registry, &depositor.registry)?;
        }

        {
            let registry = Registry::unpack(&self.registry.data.borrow())?;
            // Check manager
            assert_account_key(self.manager, &registry.manager)?;
        }

        {
            // Check rebalancing
            let (rebalancing_pubkey, _) = find_rebalancing_program_address(
                program_id,
                self.depositor.key,
                self.liquidity_mint.key,
            );
            assert_account_key(self.rebalancing, &rebalancing_pubkey)?;
        }

        let mut rebalancing = Rebalancing::unpack(&self.rebalancing.data.borrow())?;
        // Check rebalancing accounts
        assert_account_key(self.depositor, &rebalancing.depositor)?;
        assert_account_key(self.liquidity_mint, &rebalancing.mint)?;

        // Check rebalancing is not completed
        if rebalancing.is_completed() {
            return Err(EverlendError::RebalancingIsCompleted.into());
        }

        rebalancing.set(
            amount_to_distribute,
            distributed_liquidity,
            distribution_array,
        )?;

        Rebalancing::pack(rebalancing, *self.rebalancing.data.borrow_mut())?;

        Ok(())
    }
}
