use super::{CollateralStorage, MoneyMarket};
use crate::state::MiningType;
use everlend_utils::{cpi::larix, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

///
pub struct Larix<'a, 'b> {
    money_market_program_id: Pubkey,
    reserve: &'a AccountInfo<'b>,
    reserve_liquidity_supply: &'a AccountInfo<'b>,
    lending_market: &'a AccountInfo<'b>,
    lending_market_authority: &'a AccountInfo<'b>,
    reserve_liquidity_oracle: &'a AccountInfo<'b>,

    mining: Option<LarixMining<'a, 'b>>,
}

///
struct LarixMining<'a, 'b> {
    reserve_bonus: &'a AccountInfo<'b>,
    mining: &'a AccountInfo<'b>,
}

impl<'a, 'b> Larix<'a, 'b> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        internal_mining_type: Option<MiningType>,
    ) -> Result<Larix<'a, 'b>, ProgramError> {
        let reserve_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let reserve_liquidity_supply_info =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let lending_market_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let lending_market_authority_info = AccountLoader::next_unchecked(account_info_iter)?;
        let reserve_liquidity_oracle_info = AccountLoader::next_unchecked(account_info_iter)?;

        let mut larix = Larix {
            money_market_program_id,
            reserve: reserve_info,
            reserve_liquidity_supply: reserve_liquidity_supply_info,
            lending_market: lending_market_info,
            lending_market_authority: lending_market_authority_info,
            reserve_liquidity_oracle: reserve_liquidity_oracle_info,

            mining: None,
        };

        // Parse mining  accounts if presented
        match internal_mining_type {
            Some(MiningType::Larix { mining_account, .. }) => {
                let reserve_bonus_info_info =
                    AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
                let mining_info = AccountLoader::next_with_key(account_info_iter, &mining_account)?;

                larix.mining = Some(LarixMining {
                    mining: mining_info,
                    reserve_bonus: reserve_bonus_info_info,
                });
            }
            _ => {}
        }

        Ok(larix)
    }
}

impl<'a, 'b> MoneyMarket<'b> for Larix<'a, 'b> {
    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_liquidity: AccountInfo<'b>,
        destination_collateral: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        larix::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
        )?;

        larix::deposit(
            &self.money_market_program_id,
            source_liquidity,
            destination_collateral.clone(),
            self.reserve.clone(),
            self.reserve_liquidity_supply.clone(),
            collateral_mint,
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            authority,
            liquidity_amount,
            signers_seeds,
        )?;

        let collateral_amount =
            Account::unpack_unchecked(&destination_collateral.data.borrow())?.amount;

        Ok(collateral_amount)
    }

    fn money_market_redeem(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_collateral: AccountInfo<'b>,
        destination_liquidity: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        larix::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
        )?;

        larix::redeem(
            &self.money_market_program_id,
            source_collateral,
            destination_liquidity,
            self.reserve.clone(),
            collateral_mint,
            self.reserve_liquidity_supply.clone(),
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            authority,
            collateral_amount,
            signers_seeds,
        )
    }

    ///
    fn money_market_deposit_and_deposit_mining(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_liquidity: AccountInfo<'b>,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        self.money_market_deposit(
            collateral_mint,
            source_liquidity,
            collateral_transit.clone(),
            authority.clone(),
            clock.clone(),
            liquidity_amount,
            signers_seeds,
        )?;

        let collateral_amount =
            Account::unpack_unchecked(&collateral_transit.data.borrow())?.amount;

        if collateral_amount == 0 {
            return Err(EverlendError::CollateralLeak.into());
        }

        self.deposit_collateral_tokens(
            collateral_transit,
            authority,
            clock,
            collateral_amount,
            signers_seeds,
        )?;

        Ok(collateral_amount)
    }

    ///
    fn money_market_redeem_and_withdraw_mining(
        &self,
        collateral_mint: AccountInfo<'b>,
        collateral_transit: AccountInfo<'b>,
        liquidity_destination: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        self.withdraw_collateral_tokens(
            collateral_transit.clone(),
            authority.clone(),
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        self.money_market_redeem(
            collateral_mint,
            collateral_transit.clone(),
            liquidity_destination.clone(),
            authority.clone(),
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )
    }
}

impl<'a, 'b> CollateralStorage<'b> for Larix<'a, 'b> {
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        if self.mining.is_none() {
            return Err(ProgramError::InvalidArgument);
        }

        larix::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
        )?;

        let mining = self.mining.as_ref().unwrap();
        larix::deposit_mining(
            &self.money_market_program_id,
            collateral_transit,
            mining.reserve_bonus.clone(),
            mining.mining.clone(),
            self.reserve.clone(),
            self.lending_market.clone(),
            authority.clone(),
            authority,
            collateral_amount,
            signers_seeds,
        )
    }

    fn withdraw_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        if self.mining.is_none() {
            return Err(ProgramError::InvalidArgument);
        }

        larix::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
        )?;

        let mining = self.mining.as_ref().unwrap();

        larix::withdraw_mining(
            &self.money_market_program_id,
            collateral_transit,
            mining.reserve_bonus.clone(),
            mining.mining.clone(),
            self.reserve.clone(),
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            authority,
            clock,
            collateral_amount,
            signers_seeds,
        )
    }
}
