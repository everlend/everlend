use crate::find_transit_sol_unwrap_address;
use crate::money_market::MoneyMarket;
use everlend_utils::cpi;
use everlend_utils::cpi::frakt::find_deposit_address;
use everlend_utils::{assert_account_key, AccountLoader, EverlendError};
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{system_program, sysvar};
use std::iter::Enumerate;
use std::slice::Iter;

///
pub struct Frakt<'a, 'b> {
    program_id: Pubkey,
    money_market_program_id: Pubkey,
    liquidity_pool: &'a AccountInfo<'b>,
    deposit_account: &'a AccountInfo<'b>,
    liquidity_owner: &'a AccountInfo<'b>,
    admin: &'a AccountInfo<'b>,
    unwrap_sol: &'a AccountInfo<'b>,
    token_mint: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> Frakt<'a, 'b> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        program_id: Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<Frakt<'a, 'b>, ProgramError> {
        let liquidity_pool =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let liquidity_owner = AccountLoader::next_unchecked(account_info_iter)?;
        let deposit_account =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let admin = AccountLoader::next_unchecked(account_info_iter)?;
        let unwrap_sol = AccountLoader::next_uninitialized(account_info_iter)?;
        let token_mint =
            AccountLoader::next_with_key(account_info_iter, &spl_token::native_mint::id())?;
        let rent = AccountLoader::next_with_key(account_info_iter, &sysvar::rent::id())?;

        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        let frakt = Frakt {
            program_id,
            money_market_program_id,
            liquidity_pool,
            liquidity_owner,
            deposit_account,
            admin,
            unwrap_sol,
            token_mint,
            rent,
        };

        Ok(frakt)
    }
}

impl<'a, 'b> MoneyMarket<'b> for Frakt<'a, 'b> {
    fn money_market_deposit(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _source_liquidity: AccountInfo<'b>,
        _destination_collateral: AccountInfo<'b>,
        _authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        _liquidity_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        return Err(EverlendError::MiningIsRequired.into());
    }

    fn money_market_redeem(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _source_collateral: AccountInfo<'b>,
        _destination_liquidity: AccountInfo<'b>,
        _authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        _collateral_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        return Err(EverlendError::MiningIsRequired.into());
    }

    fn money_market_deposit_and_deposit_mining(
        &self,
        _collateral_mint: AccountInfo<'b>,
        source_liquidity: AccountInfo<'b>,
        _collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        let unwrap_acc_signers_seeds = {
            let (unwrap_sol_pubkey, bump_seed) =
                find_transit_sol_unwrap_address(&self.program_id, self.liquidity_pool.key);
            assert_account_key(self.unwrap_sol, &unwrap_sol_pubkey)?;

            &[
                br"unwrap",
                &self.liquidity_pool.key.to_bytes()[..32],
                &[bump_seed],
            ]
        };

        cpi::system::create_account::<spl_token::state::Account>(
            &spl_token::id(),
            authority.clone(),
            self.unwrap_sol.clone(),
            &[unwrap_acc_signers_seeds],
            &Rent::from_account_info(self.rent)?,
        )?;
        cpi::spl_token::initialize_account(
            self.unwrap_sol.clone(),
            self.token_mint.clone(),
            authority.clone(),
            self.rent.clone(),
        )?;

        cpi::spl_token::transfer(
            source_liquidity,
            self.unwrap_sol.clone(),
            authority.clone(),
            liquidity_amount,
            signers_seeds,
        )?;

        cpi::spl_token::close_account(
            authority.clone(),
            self.unwrap_sol.clone(),
            authority.clone(),
            signers_seeds,
        )?;

        cpi::frakt::deposit(
            &self.money_market_program_id,
            self.liquidity_pool.clone(),
            self.liquidity_owner.clone(),
            self.deposit_account.clone(),
            authority,
            self.rent.clone(),
            liquidity_amount,
            signers_seeds,
        )?;

        Ok(liquidity_amount)
    }

    fn money_market_redeem_and_withdraw_mining(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _collateral_transit: AccountInfo<'b>,
        liquidity_destination: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        let (_, dump) =
            find_deposit_address(&self.program_id, self.liquidity_pool.key, authority.key);

        cpi::frakt::redeem(
            &self.money_market_program_id,
            self.liquidity_pool.clone(),
            self.deposit_account.clone(),
            authority.clone(),
            self.liquidity_owner.clone(),
            self.admin.clone(),
            collateral_amount,
            dump,
            signers_seeds,
        )?;

        cpi::system::transfer(
            authority.clone(),
            liquidity_destination.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        cpi::spl_token::sync_native(liquidity_destination.clone())?;

        Ok(())
    }
}