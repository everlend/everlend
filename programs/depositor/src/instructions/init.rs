use crate::state::{Depositor, InitDepositorParams};
use everlend_utils::{assert_rent_exempt, assert_uninitialized, AccountLoader};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey, rent::Rent, sysvar::Sysvar, sysvar::SysvarId,
};
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct InitContext<'a, 'b> {
    depositor: &'a AccountInfo<'b>,
    registry: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<InitContext<'a, 'b>, ProgramError> {
        let depositor = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let registry = AccountLoader::next_with_owner(account_info_iter, &everlend_registry::id())?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;

        Ok(InitContext {
            depositor,
            registry,
            rent,
        })
    }

    /// Process Init instruction
    pub fn process(
        &self,
        _program_id: &Pubkey,
        _account_info_iter: &'a mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        rebalance_executor: Pubkey,
    ) -> ProgramResult {
        let rent = &Rent::from_account_info(self.rent)?;

        assert_rent_exempt(rent, self.depositor)?;

        // Get depositor state
        let mut depositor = Depositor::unpack_unchecked(&self.depositor.data.borrow())?;
        assert_uninitialized(&depositor)?;

        depositor.init(InitDepositorParams {
            registry: *self.registry.key,
            rebalance_executor,
        });

        Depositor::pack(depositor, *self.depositor.data.borrow_mut())?;
        Ok(())
    }
}
