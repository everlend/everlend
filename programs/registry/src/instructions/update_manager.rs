use everlend_utils::{assert_account_key, next_account, next_signer_account};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

use crate::state::Registry;

/// Instruction context
pub struct UpdateManagerContext<'a, 'b> {
    manager: &'a AccountInfo<'b>,
    new_manager: &'a AccountInfo<'b>,
    registry: &'a AccountInfo<'b>,
}

impl<'a, 'b> UpdateManagerContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<UpdateManagerContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        let registry_info = next_account(account_info_iter, program_id)?;
        let manager_info = next_signer_account(account_info_iter)?;
        let new_manager_info = next_signer_account(account_info_iter)?;

        Ok(UpdateManagerContext {
            manager: manager_info,
            new_manager: new_manager_info,
            registry: registry_info,
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey) -> ProgramResult {
        let mut r = Registry::unpack(&self.registry.data.borrow())?;
        assert_account_key(&self.manager, &r.manager)?;

        r.manager = *self.new_manager.key;
        Registry::pack(r, *self.registry.data.borrow_mut())?;

        Ok(())
    }
}
