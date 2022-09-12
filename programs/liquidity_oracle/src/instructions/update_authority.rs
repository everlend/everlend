use everlend_utils::{assert_account_key, AccountLoader};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

use crate::state::LiquidityOracle;

/// Instruction context
pub struct UpdateAuthorityContext<'a, 'b> {
    liquidity_oracle: &'a AccountInfo<'b>,
    new_authority: &'a AccountInfo<'b>,
    authority: &'a AccountInfo<'b>,
}

impl<'a, 'b> UpdateAuthorityContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<UpdateAuthorityContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let liquidity_oracle = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let new_authority = AccountLoader::next_unchecked(account_info_iter)?; // can be any account
        let authority = AccountLoader::next_signer(account_info_iter)?;

        Ok(UpdateAuthorityContext {
            liquidity_oracle,
            new_authority,
            authority,
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey) -> ProgramResult {
        let mut liquidity_oracle = LiquidityOracle::unpack(&self.liquidity_oracle.data.borrow())?;

        // Check current authority
        assert_account_key(self.authority, &liquidity_oracle.authority)?;

        // Update to new authority
        liquidity_oracle.update(*self.new_authority.key);

        LiquidityOracle::pack(liquidity_oracle, *self.liquidity_oracle.data.borrow_mut())?;

        Ok(())
    }
}
