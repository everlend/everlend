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
pub struct InitContext<'a> {
    manager: AccountInfo<'a>,
    registry: AccountInfo<'a>,
    rent: AccountInfo<'a>,
}

impl<'a> InitContext<'a> {
    /// New instruction context
    pub fn new(
        _program_id: &Pubkey,
        accounts: &[AccountInfo<'a>],
    ) -> Result<InitContext<'a>, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let registry_info = next_uninitialized_account(account_info_iter)?;
        let manager_info = next_signer_account(account_info_iter)?;
        let _system_info = next_program_account(account_info_iter, &system_program::id())?;
        let rent_info = next_program_account(account_info_iter, &Rent::id())?;

        Ok(InitContext {
            manager: manager_info.clone(),
            registry: registry_info.clone(),
            rent: rent_info.clone(),
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
