use crate::find_transit_sol_unwrap_address;
use crate::money_market::MoneyMarket;
use everlend_utils::cpi;
use everlend_utils::cpi::frakt::{find_deposit_address, find_owner_address};
use everlend_utils::{AccountLoader, EverlendError};
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
        authority: &Pubkey,
        token_mint: &'a AccountInfo<'b>,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<Frakt<'a, 'b>, ProgramError> {
        let liquidity_pool =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;

        let liquidity_owner = {
            let (liquidity_owner_pubkey, _) =
                find_owner_address(&money_market_program_id, liquidity_pool.key);
            AccountLoader::next_with_key(account_info_iter, &liquidity_owner_pubkey)?
        };
        let deposit_account = {
            let (deposit_account_pubkey, _) =
                find_deposit_address(&money_market_program_id, liquidity_pool.key, authority);
            AccountLoader::next_with_key(account_info_iter, &deposit_account_pubkey)?
        };
        let admin = AccountLoader::next_unchecked(account_info_iter)?;

        let unwrap_sol = {
            let (unwrap_sol_pubkey, _) =
                find_transit_sol_unwrap_address(&program_id, liquidity_pool.key);
            AccountLoader::next_with_key(account_info_iter, &unwrap_sol_pubkey)?
        };
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
    fn is_collateral_return(&self) -> bool {
        false
    }

    fn money_market_deposit(
        &self,
        _collateral_mint: AccountInfo<'b>,
        source_liquidity: AccountInfo<'b>,
        _destination_collateral: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        let unwrap_acc_signers_seeds = {
            let (_, bump_seed) =
                find_transit_sol_unwrap_address(&self.program_id, self.liquidity_pool.key);

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

        // Return 0 because FRAKT doesn't return collateral tokens
        Ok(0)
    }

    fn money_market_redeem(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _source_collateral: AccountInfo<'b>,
        destination_liquidity: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        let (_, dump) = find_deposit_address(
            &self.money_market_program_id,
            self.liquidity_pool.key,
            authority.key,
        );

        let starting_lamports = authority.lamports();

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

        cpi::frakt::claim_rewards(
            &self.money_market_program_id,
            self.liquidity_pool.clone(),
            self.deposit_account.clone(),
            authority.clone(),
            self.liquidity_owner.clone(),
            self.admin.clone(),
            dump,
            &signers_seeds,
        )?;

        let amount = authority
            .lamports()
            .checked_sub(starting_lamports)
            .ok_or(EverlendError::MathOverflow)?;

        cpi::system::transfer(
            authority.clone(),
            destination_liquidity.clone(),
            amount,
            signers_seeds,
        )?;

        cpi::spl_token::sync_native(destination_liquidity.clone())?;

        Ok(())
    }

    fn money_market_deposit_and_deposit_mining(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _source_liquidity: AccountInfo<'b>,
        _collateral_transit: AccountInfo<'b>,
        _authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        _liquidity_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        Err(EverlendError::MiningNotImplemented.into())
    }

    fn money_market_redeem_and_withdraw_mining(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _collateral_transit: AccountInfo<'b>,
        _liquidity_destination: AccountInfo<'b>,
        _authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        _collateral_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        Err(EverlendError::MiningNotImplemented.into())
    }

    fn is_income(
        &self,
        _collateral_amount: u64,
        _expected_liquidity_amount: u64,
    ) -> Result<bool, ProgramError> {
        Ok(true)
    }

    fn refresh_reserve(&self, _clock: AccountInfo<'b>) -> Result<(), ProgramError> {
        Ok(())
    }
}
