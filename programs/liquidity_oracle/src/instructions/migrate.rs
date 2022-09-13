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
    find_token_distribution_program_address,
    state::{DeprecatedTokenDistribution, DistributionArray, LiquidityOracle, TokenDistribution},
};

/// Instruction context
pub struct MigrateContext<'a, 'b> {
    liquidity_oracle: &'a AccountInfo<'b>,
    token_mint: &'a AccountInfo<'b>,
    token_distribution: &'a AccountInfo<'b>,
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
        let token_distribution = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let authority = AccountLoader::next_signer(account_info_iter)?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        Ok(MigrateContext {
            liquidity_oracle,
            token_mint,
            token_distribution,
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
            let (token_distribution_pubkey, _) = find_token_distribution_program_address(
                program_id,
                self.liquidity_oracle.key,
                self.token_mint.key,
            );

            assert_account_key(self.token_distribution, &token_distribution_pubkey)?;
        }

        let deprecated =
            DeprecatedTokenDistribution::unpack(&self.token_distribution.data.borrow())?;

        // Realloc account
        realloc_with_rent(
            self.token_distribution,
            self.authority,
            &Rent::from_account_info(self.rent)?,
            TokenDistribution::LEN,
        )?;

        let mut distribution = TokenDistribution::init();
        distribution.distribution = deprecated.distribution;
        distribution.reserve_rates = DistributionArray::default();
        distribution.updated_at = deprecated.updated_at;

        TokenDistribution::pack(distribution, *self.token_distribution.data.borrow_mut())?;

        Ok(())
    }
}
