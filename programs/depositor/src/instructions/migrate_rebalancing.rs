use crate::state::{Depositor, DeprecatedRebalancing, Rebalancing};
use everlend_registry::state::Registry;
use everlend_utils::cpi::system::realloc_with_rent;
use everlend_utils::{assert_account_key, AccountLoader};
use solana_program::program_pack::Pack;
use solana_program::rent::Rent;
use solana_program::sysvar::{Sysvar, SysvarId};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey, system_program,
};
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct MigrateRebalancingContext<'a, 'b> {
    rebalancing: &'a AccountInfo<'b>,
    depositor: &'a AccountInfo<'b>,
    registry: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> MigrateRebalancingContext<'a, 'b> {
    /// New MigrateRebalancing instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<MigrateRebalancingContext<'a, 'b>, ProgramError> {
        let rebalancing = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let depositor = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let registry = AccountLoader::next_with_owner(account_info_iter, &everlend_registry::id())?;
        let manager = AccountLoader::next_signer(account_info_iter)?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        Ok(MigrateRebalancingContext {
            rebalancing,
            depositor,
            registry,
            manager,
            rent,
        })
    }

    /// Process MigrateRebalancing instruction
    pub fn process(
        &self,
        _program_id: &Pubkey,
        _account_info_iter: &'a mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> ProgramResult {
        let rebalancing = DeprecatedRebalancing::unpack(&self.rebalancing.data.borrow())?;
        assert_account_key(self.depositor, &rebalancing.depositor)?;

        // Check manager
        {
            let depositor = Depositor::unpack(&self.depositor.data.borrow())?;
            assert_account_key(self.registry, &depositor.registry)?;
            let registry = Registry::unpack(&self.registry.data.borrow())?;
            assert_account_key(self.manager, &registry.manager)?;
        }

        let rebalancing = Rebalancing {
            account_type: rebalancing.account_type,
            depositor: rebalancing.depositor,
            mint: rebalancing.mint,
            amount_to_distribute: rebalancing.amount_to_distribute,
            distributed_liquidity: rebalancing.distributed_liquidity,
            received_collateral: rebalancing.received_collateral,
            liquidity_distribution: rebalancing.liquidity_distribution,
            steps: rebalancing.steps,
            income_refreshed_at: rebalancing.income_refreshed_at,
        };

        realloc_with_rent(
            self.rebalancing,
            self.manager,
            &Rent::from_account_info(self.rent)?,
            Rebalancing::LEN,
        )?;

        Rebalancing::pack(rebalancing, *self.rebalancing.data.borrow_mut())?;

        Ok(())
    }
}
