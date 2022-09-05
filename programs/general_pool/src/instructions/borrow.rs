use crate::{
    state::{Pool, PoolBorrowAuthority},
    utils::total_pool_amount,
};
use everlend_utils::{
    assert_account_key, cpi, find_program_address, next_account, next_program_account,
    next_signer_account,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

/// Instruction context
pub struct BorrowContext<'a, 'b> {
    pool_market: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    pool_borrow_authority: &'a AccountInfo<'b>,
    destination: &'a AccountInfo<'b>,
    token_account: &'a AccountInfo<'b>,
    pool_market_authority: &'a AccountInfo<'b>,
    borrow_authority: &'a AccountInfo<'b>,
}

impl<'a, 'b> BorrowContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<BorrowContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        let pool_market = next_account(account_info_iter, program_id)?;
        let pool = next_account(account_info_iter, program_id)?;
        let pool_borrow_authority = next_account(account_info_iter, program_id)?;
        let destination = next_account(account_info_iter, &spl_token::id())?;
        let token_account = next_account(account_info_iter, &spl_token::id())?;
        let pool_market_authority = next_account(account_info_iter, program_id)?;
        let borrow_authority = next_signer_account(account_info_iter)?;
        let _token_program = next_program_account(account_info_iter, &spl_token::id())?;

        Ok(BorrowContext {
            pool_market,
            pool,
            pool_borrow_authority,
            destination,
            token_account,
            pool_market_authority,
            borrow_authority,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey, amount: u64) -> ProgramResult {
        let mut pool = Pool::unpack(&self.pool.data.borrow())?;

        // Check pool accounts
        assert_account_key(self.pool_market, &pool.pool_market)?;
        assert_account_key(self.token_account, &pool.token_account)?;

        let mut pool_borrow_authority =
            PoolBorrowAuthority::unpack(&self.pool_borrow_authority.data.borrow())?;

        // Check pool borrow authority accounts
        assert_account_key(self.pool, &pool_borrow_authority.pool)?;
        assert_account_key(
            self.borrow_authority,
            &pool_borrow_authority.borrow_authority,
        )?;

        pool_borrow_authority.borrow(amount)?;
        pool_borrow_authority.check_amount_allowed(total_pool_amount(
            self.token_account.clone(),
            pool.total_amount_borrowed,
        )?)?;
        pool.borrow(amount)?;

        // Check interest ?

        PoolBorrowAuthority::pack(
            pool_borrow_authority,
            *self.pool_borrow_authority.data.borrow_mut(),
        )?;

        Pool::pack(pool, *self.pool.data.borrow_mut())?;

        let (_, bump_seed) = find_program_address(program_id, self.pool_market.key);
        let signers_seeds = &[&self.pool_market.key.to_bytes()[..32], &[bump_seed]];

        // Transfer from token account to destination borrower
        cpi::spl_token::transfer(
            self.token_account.clone(),
            self.destination.clone(),
            self.pool_market_authority.clone(),
            amount,
            &[signers_seeds],
        )?;

        Ok(())
    }
}
