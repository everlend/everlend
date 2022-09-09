use everlend_utils::{cpi, AccountLoader};
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
    registry: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<InitContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let registry = AccountLoader::next_uninitialized(account_info_iter)?;
        let manager = AccountLoader::next_signer(account_info_iter)?;
        let _system = AccountLoader::next_with_key(account_info_iter, &system_program::id())?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;

        Ok(InitContext {
            registry,
            manager,
            rent,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let rent = &Rent::from_account_info(self.rent)?;

        cpi::system::create_account::<Registry>(
            program_id,
            self.manager.clone(),
            self.registry.clone(),
            &[],
            rent,
        )?;

        let r = Registry::init(*self.manager.key);
        Registry::pack(r, *self.registry.data.borrow_mut())?;

        Ok(())
    }
}
