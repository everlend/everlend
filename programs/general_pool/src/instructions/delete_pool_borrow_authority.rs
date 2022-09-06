use crate::state::{Pool, PoolBorrowAuthority, PoolMarket};
use everlend_utils::{assert_account_key, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

/// Instruction context
pub struct DeletePoolBorrowAuthorityContext<'a, 'b> {
    pool_market: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    pool_borrow_authority: &'a AccountInfo<'b>,
    receiver: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
}

impl<'a, 'b> DeletePoolBorrowAuthorityContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<DeletePoolBorrowAuthorityContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let pool_market = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let pool_borrow_authority = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let receiver = AccountLoader::next_unchecked(account_info_iter)?;
        let manager = AccountLoader::next_signer(account_info_iter)?;

        Ok(DeletePoolBorrowAuthorityContext {
            pool_market,
            pool,
            pool_borrow_authority,
            receiver,
            manager,
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey) -> ProgramResult {
        // Check manager
        {
            let pool_market = PoolMarket::unpack(&self.pool_market.data.borrow())?;
            assert_account_key(self.manager, &pool_market.manager)?;

            // Get pool state
            let pool = Pool::unpack(&self.pool.data.borrow())?;
            assert_account_key(self.pool_market, &pool.pool_market)?;

            // Get pool borrow authority state to check initialized
            let pool_borrow_authority =
                PoolBorrowAuthority::unpack(&self.pool_borrow_authority.data.borrow())?;
            assert_account_key(self.pool, &pool_borrow_authority.pool)?;
        }

        let receiver_starting_lamports = self.receiver.lamports();
        let pool_borrow_authority_lamports = self.pool_borrow_authority.lamports();

        **self.pool_borrow_authority.lamports.borrow_mut() = 0;
        **self.receiver.lamports.borrow_mut() = receiver_starting_lamports
            .checked_add(pool_borrow_authority_lamports)
            .ok_or(EverlendError::MathOverflow)?;

        PoolBorrowAuthority::pack(
            Default::default(),
            *self.pool_borrow_authority.data.borrow_mut(),
        )?;

        Ok(())
    }
}
