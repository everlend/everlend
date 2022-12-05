
use everlend_utils::{assert_account_key, find_program_address, AccountLoader, EverlendError, PDA};
use solana_program::{
    account_info::AccountInfo, pubkey::Pubkey, program_error::ProgramError, sysvar::clock, sysvar::Sysvar,
    entrypoint::ProgramResult,
};
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct DepositContext<'a, 'b> {
    depositor_authority: &'a AccountInfo<'b>,

    collateral_transit: &'a AccountInfo<'b>,
    collateral_mint: &'a AccountInfo<'b>,

    liquidity_transit: &'a AccountInfo<'b>,

    clock: &'a AccountInfo<'b>,

    money_market_program: &'a AccountInfo<'b>,
}

impl<'a, 'b> DepositContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<DepositContext<'a, 'b>, ProgramError> {

        let depositor_authority = AccountLoader::next_unchecked(account_info_iter)?; //Signer PDA
        let liquidity_transit =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let collateral_transit =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let collateral_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let clock = AccountLoader::next_with_key(account_info_iter, &clock::id())?;
        let money_market_program = AccountLoader::next_unchecked(account_info_iter)?;


        Ok(DepositContext {
            depositor_authority,
            collateral_transit,
            collateral_mint,
            liquidity_transit,
            money_market_program,
            clock,
        })
    }

    /// Process instruction
    pub fn process(
        &self,
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> ProgramResult {


        Ok(())
    }
}
