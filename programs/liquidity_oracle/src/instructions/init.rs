use everlend_utils::{assert_uninitialized, AccountLoader};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

use crate::state::LiquidityOracle;

/// Instruction context
pub struct InitContext<'a, 'b> {
    liquidity_oracle: &'a AccountInfo<'b>,
    authority: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<InitContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let liquidity_oracle = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let authority = AccountLoader::next_signer(account_info_iter)?;

        Ok(InitContext {
            liquidity_oracle,
            authority,
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey) -> ProgramResult {
        let liquidity_oracle =
            LiquidityOracle::unpack_unchecked(&self.liquidity_oracle.data.borrow())?;
        assert_uninitialized(&liquidity_oracle)?;

        // Initialize
        let liquidity_oracle = LiquidityOracle::init(*self.authority.key);
        LiquidityOracle::pack(liquidity_oracle, *self.liquidity_oracle.data.borrow_mut())?;

        Ok(())
    }
}
