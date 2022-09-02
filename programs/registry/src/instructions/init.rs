use everlend_utils::{cpi, next_program_account, next_signer_account, next_uninitialized_account};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_program,
    sysvar::{Sysvar, SysvarId},
};

use crate::state::Registry;

/// Instruction context
pub struct InitContext<'a, 'b> {
    manager: &'a AccountInfo<'b>,
    registry: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<InitContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let registry_info = next_uninitialized_account(account_info_iter)?;
        let manager_info = next_signer_account(account_info_iter)?;
        let _system_info = next_program_account(account_info_iter, &system_program::id())?;
        let rent_info = next_program_account(account_info_iter, &Rent::id())?;

        Ok(InitContext {
            manager: manager_info,
            registry: registry_info,
            rent: rent_info,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let rent = &Rent::from_account_info(&self.rent)?;

        cpi::system::create_account::<Registry>(
            program_id,
            self.manager.clone(),
            self.registry.clone(),
            &[],
            rent,
        )?;

        let r = Registry::init(self.manager.key.clone());
        Registry::pack(r, *self.registry.data.borrow_mut())?;

        Ok(())
    }
}
