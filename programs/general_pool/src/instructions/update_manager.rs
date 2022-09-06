use crate::state::PoolMarket;
use everlend_utils::{assert_account_key, next_account, next_signer_account};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

/// Instruction context
pub struct UpdateManagerContext<'a, 'b> {
    pool_market: &'a AccountInfo<'b>,
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

        let pool_market = next_account(account_info_iter, program_id)?;
        let manager = next_signer_account(account_info_iter)?;
        let new_manager = next_signer_account(account_info_iter)?;

        Ok(UpdateManagerContext {
            pool_market,
            manager,
            new_manager,
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey) -> ProgramResult {
        let mut pool_market = PoolMarket::unpack(&self.pool_market.data.borrow())?;
        assert_account_key(self.manager, &pool_market.manager)?;

        pool_market.manager = *self.new_manager.key;

        PoolMarket::pack(pool_market, *self.pool_market.data.borrow_mut())?;

        Ok(())
    }
}
