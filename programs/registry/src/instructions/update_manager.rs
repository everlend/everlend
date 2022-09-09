use everlend_utils::{assert_account_key, AccountLoader};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

use crate::state::Registry;

/// Instruction context
pub struct UpdateManagerContext<'a, 'b> {
    registry: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
    new_manager: &'a AccountInfo<'b>,
}

impl<'a, 'b> UpdateManagerContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<UpdateManagerContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        let registry = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let manager = AccountLoader::next_signer(account_info_iter)?;
        let new_manager = AccountLoader::next_signer(account_info_iter)?;

        Ok(UpdateManagerContext {
            registry,
            manager,
            new_manager,
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey) -> ProgramResult {
        let mut r = Registry::unpack(&self.registry.data.borrow())?;
        assert_account_key(self.manager, &r.manager)?;

        r.manager = *self.new_manager.key;
        Registry::pack(r, *self.registry.data.borrow_mut())?;

        Ok(())
    }
}
