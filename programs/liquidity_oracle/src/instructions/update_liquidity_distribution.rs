use everlend_utils::{assert_account_key, AccountLoader};
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::{Sysvar, SysvarId},
};

use crate::{
    find_token_oracle_program_address,
    state::{DistributionArray, LiquidityOracle, TokenOracle},
};

/// Instruction context
pub struct UpdateLiquidityDistributionContext<'a, 'b> {
    liquidity_oracle: &'a AccountInfo<'b>,
    token_mint: &'a AccountInfo<'b>,
    token_oracle: &'a AccountInfo<'b>,
    authority: &'a AccountInfo<'b>,
    clock: &'a AccountInfo<'b>,
}

impl<'a, 'b> UpdateLiquidityDistributionContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<UpdateLiquidityDistributionContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let liquidity_oracle = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let token_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let token_oracle = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let authority = AccountLoader::next_signer(account_info_iter)?;
        let clock = AccountLoader::next_with_key(account_info_iter, &Clock::id())?;

        Ok(UpdateLiquidityDistributionContext {
            liquidity_oracle,
            token_mint,
            token_oracle,
            authority,
            clock,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey, distribution: DistributionArray) -> ProgramResult {
        {
            // Check authotiry
            let liquidity_oracle = LiquidityOracle::unpack(&self.liquidity_oracle.data.borrow())?;
            assert_account_key(self.authority, &liquidity_oracle.authority)?;

            // Check token distribution
            let (token_oracle_pubkey, _) = find_token_oracle_program_address(
                program_id,
                self.liquidity_oracle.key,
                self.token_mint.key,
            );

            assert_account_key(self.token_oracle, &token_oracle_pubkey)?;
        }

        let clock = Clock::from_account_info(self.clock)?;

        let mut oracle = TokenOracle::unpack(&self.token_oracle.data.borrow())?;
        oracle.update_liquidity_distribution(clock.slot, distribution)?;

        TokenOracle::pack(oracle, *self.token_oracle.data.borrow_mut())?;

        Ok(())
    }
}
