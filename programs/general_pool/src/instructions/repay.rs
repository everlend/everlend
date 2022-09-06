use crate::state::{Pool, PoolBorrowAuthority};
use everlend_utils::{assert_account_key, cpi, AccountLoader};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

/// Instruction context
pub struct RepayContext<'a, 'b> {
    pool_market: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    pool_borrow_authority: &'a AccountInfo<'b>,
    source: &'a AccountInfo<'b>,
    token_account: &'a AccountInfo<'b>,
    user_transfer_authority: &'a AccountInfo<'b>,
}

impl<'a, 'b> RepayContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<RepayContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let pool_market = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let pool_borrow_authority = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let source = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let token_account = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let user_transfer_authority = AccountLoader::next_signer(account_info_iter)?;
        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;

        Ok(RepayContext {
            pool_market,
            pool,
            pool_borrow_authority,
            source,
            token_account,
            user_transfer_authority,
        })
    }

    /// Process instruction
    pub fn process(
        &self,
        _program_id: &Pubkey,
        amount: u64,
        interest_amount: u64,
    ) -> ProgramResult {
        // Get pool state
        let mut pool = Pool::unpack(&self.pool.data.borrow())?;

        // Check pool accounts
        assert_account_key(self.pool_market, &pool.pool_market)?;
        assert_account_key(self.token_account, &pool.token_account)?;

        // Get pool borrow authority state
        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&self.pool_borrow_authority.data.borrow())?;
        assert_account_key(self.pool, &pool_borrow_authority.pool)?;

        pool_borrow_authority.repay(amount)?;
        pool.repay(amount)?;

        // Check interest ?

        PoolBorrowAuthority::pack(
            pool_borrow_authority,
            *self.pool_borrow_authority.data.borrow_mut(),
        )?;
        Pool::pack(pool, *self.pool.data.borrow_mut())?;

        // Transfer from source to token account
        cpi::spl_token::transfer(
            self.source.clone(),
            self.token_account.clone(),
            self.user_transfer_authority.clone(),
            amount + interest_amount,
            &[],
        )?;

        Ok(())
    }
}
