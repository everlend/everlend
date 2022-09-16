use everlend_utils::{assert_account_key, cpi::system::realloc_with_rent, AccountLoader};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_program,
    sysvar::{Sysvar, SysvarId},
};

use crate::{
    find_token_oracle_program_address,
    state::{DeprecatedTokenDistribution, DistributionArray, LiquidityOracle, TokenOracle},
};

/// Instruction context
pub struct MigrateContext<'a, 'b> {
    liquidity_oracle: &'a AccountInfo<'b>,
    token_mint: &'a AccountInfo<'b>,
    token_oracle: &'a AccountInfo<'b>,
    authority: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> MigrateContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<MigrateContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let liquidity_oracle = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let token_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let token_oracle = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let authority = AccountLoader::next_signer(account_info_iter)?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        Ok(MigrateContext {
            liquidity_oracle,
            token_mint,
            token_oracle,
            authority,
            rent,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
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

        let deprecated = DeprecatedTokenDistribution::unpack(&self.token_oracle.data.borrow())?;

        // Realloc account
        realloc_with_rent(
            self.token_oracle,
            self.authority,
            &Rent::from_account_info(self.rent)?,
            TokenOracle::LEN,
        )?;

        let mut oracle = TokenOracle::init();
        oracle.update_liquidity_distribution(deprecated.updated_at, deprecated.distribution)?;
        oracle.update_reserve_rates(deprecated.updated_at, DistributionArray::default())?;

        TokenOracle::pack(oracle, *self.token_oracle.data.borrow_mut())?;

        Ok(())
    }
}
