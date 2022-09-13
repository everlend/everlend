use everlend_utils::{assert_account_key, cpi::system::create_account, AccountLoader};
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
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
    state::{DistributionArray, LiquidityOracle, TokenDistribution},
};

/// Instruction context
pub struct CreateTokenDistributionContext<'a, 'b> {
    liquidity_oracle: &'a AccountInfo<'b>,
    token_mint: &'a AccountInfo<'b>,
    token_distribution: &'a AccountInfo<'b>,
    authority: &'a AccountInfo<'b>,
    clock: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> CreateTokenDistributionContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<CreateTokenDistributionContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let liquidity_oracle = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let token_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let token_distribution = AccountLoader::next_uninitialized(account_info_iter)?;
        let authority = AccountLoader::next_signer(account_info_iter)?;
        let clock = AccountLoader::next_with_key(account_info_iter, &Clock::id())?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;
        let _system = AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        Ok(CreateTokenDistributionContext {
            liquidity_oracle,
            token_mint,
            token_distribution,
            authority,
            clock,
            rent,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey, data: DistributionArray) -> ProgramResult {
        {
            // Check authotiry
            let liquidity_oracle = LiquidityOracle::unpack(&self.liquidity_oracle.data.borrow())?;
            assert_account_key(self.authority, &liquidity_oracle.authority)?;
        }

        let bump_seed = {
            let (token_distribution_pubkey, bump_seed) = find_token_distribution_program_address(
                program_id,
                self.liquidity_oracle.key,
                self.token_mint.key,
            );

            // msg!("Token distribution provided does not match generated token distribution");
            assert_account_key(self.token_distribution, &token_distribution_pubkey)?;

            bump_seed
        };

        let rent = &Rent::from_account_info(self.rent)?;
        let clock = Clock::from_account_info(self.clock)?;
        let mut distribution = TokenDistribution::init();

        let signers_seeds = &[
            &self.liquidity_oracle.key.to_bytes()[..32],
            &self.token_mint.key.to_bytes()[..32],
            &[bump_seed],
        ];

        // Create distribution storage account
        create_account::<TokenDistribution>(
            program_id,
            self.authority.clone(),
            self.token_distribution.clone(),
            &[signers_seeds],
            rent,
        )?;

        distribution.update_distribution(clock.slot, data)?;

        TokenDistribution::pack(distribution, *self.token_distribution.data.borrow_mut())?;

        Ok(())
    }
}
