use crate::state::RewardsRoot;
use everlend_utils::AccountLoader;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint_deprecated::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

/// Instruction context
pub struct MigrateRootContext<'a, 'b> {
    rewards_root: &'a AccountInfo<'b>,
    payer: &'a AccountInfo<'b>,
}

impl<'a, 'b> MigrateRootContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<MigrateRootContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let rewards_root = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let payer = AccountLoader::next_signer(account_info_iter)?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        Ok(MigrateRootContext {
            rewards_root,
            payer,
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey) -> ProgramResult {
        let rewards_root = RewardsRoot::init(*self.payer.key);

        RewardsRoot::pack(rewards_root, *self.rewards_root.data.borrow_mut())?;

        Ok(())
    }
}
