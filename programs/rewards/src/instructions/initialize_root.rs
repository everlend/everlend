use crate::state::RewardsRoot;
use everlend_utils::{assert_signer, AccountLoader};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint_deprecated::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_program;
use solana_program::sysvar::{Sysvar, SysvarId};

/// Instruction context
pub struct InitializeRootContext<'a, 'b> {
    rewards_root: &'a AccountInfo<'b>,
    payer: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitializeRootContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<InitializeRootContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let rewards_root = AccountLoader::next_uninitialized(account_info_iter)?;
        let payer = AccountLoader::next_signer(account_info_iter)?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;

        Ok(InitializeRootContext {
            rewards_root,
            payer,
            rent,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let rent = Rent::from_account_info(self.rent)?;

        assert_signer(self.rewards_root)?;

        everlend_utils::cpi::system::create_account::<RewardsRoot>(
            program_id,
            self.payer.clone(),
            self.rewards_root.clone(),
            &[],
            &rent,
        )?;
        let rewards_root = RewardsRoot::init(*self.payer.key);
        RewardsRoot::pack(rewards_root, *self.rewards_root.data.borrow_mut())?;

        Ok(())
    }
}
