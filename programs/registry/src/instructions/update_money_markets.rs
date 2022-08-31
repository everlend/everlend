use everlend_utils::{assert_account_key, next_account, next_signer_account};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

use crate::state::{DistributionPubkeys, Registry};

/// Instruction context
pub struct UpdateMoneyMarketsContext<'a> {
    manager: AccountInfo<'a>,
    registry: AccountInfo<'a>,
}

impl<'a> UpdateMoneyMarketsContext<'a> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'a>],
    ) -> Result<UpdateMoneyMarketsContext<'a>, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        let registry_info = next_account(account_info_iter, program_id)?;
        let manager_info = next_signer_account(account_info_iter)?;

        Ok(UpdateMoneyMarketsContext {
            manager: manager_info.clone(),
            registry: registry_info.clone(),
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey, data: DistributionPubkeys) -> ProgramResult {
        let mut r = Registry::unpack(&self.registry.data.borrow())?;
        assert_account_key(&self.manager, &r.manager)?;

        r.money_market_program_ids = data;

        Registry::pack(r, *self.registry.data.borrow_mut())?;

        Ok(())
    }
}
