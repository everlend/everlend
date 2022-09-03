use crate::state::{Pool, PoolBorrowAuthority, PoolMarket};
use everlend_utils::{assert_account_key, next_account, next_signer_account};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

/// Instruction context
pub struct UpdatePoolBorrowAuthorityContext<'a, 'b> {
    pool_market: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    pool_borrow_authority: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
}

impl<'a, 'b> UpdatePoolBorrowAuthorityContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<UpdatePoolBorrowAuthorityContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let pool_market = next_account(account_info_iter, program_id)?;
        let pool = next_account(account_info_iter, program_id)?;
        let pool_borrow_authority = next_account(account_info_iter, program_id)?;
        let manager = next_signer_account(account_info_iter)?;

        Ok(UpdatePoolBorrowAuthorityContext {
            pool_market,
            pool,
            pool_borrow_authority,
            manager,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey, share_allowed: u16) -> ProgramResult {
        // Check manager
        {
            let pool_market = PoolMarket::unpack(&self.pool_market.data.borrow())?;
            assert_account_key(&self.manager, &pool_market.manager)?;

            // Get pool state
            let pool = Pool::unpack(&self.pool.data.borrow())?;
            assert_account_key(self.pool_market, &pool.pool_market)?;
        }

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&self.pool_borrow_authority.data.borrow())?;
        assert_account_key(self.pool, &pool_borrow_authority.pool)?;

        pool_borrow_authority.share_allowed = share_allowed;

        PoolBorrowAuthority::pack(
            pool_borrow_authority,
            *self.pool_borrow_authority.data.borrow_mut(),
        )?;

        Ok(())
    }
}
