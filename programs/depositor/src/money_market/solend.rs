use super::MoneyMarket;
use crate::money_market::CollateralStorage;
use crate::state::MiningType;
use everlend_utils::{assert_account_key, cpi::solend, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

///
pub struct Solend<'a> {
    money_market_program_id: Pubkey,
    reserve: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    reserve_liquidity_pyth_oracle: AccountInfo<'a>,
    reserve_liquidity_switchboard_oracle: AccountInfo<'a>,

    mining: Option<SolendMining<'a>>,
}

struct SolendMining<'a> {
    obligation_info: AccountInfo<'a>,
    collateral_supply_info: AccountInfo<'a>,
}

impl<'a, 'b> Solend<'a> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        account_info_iter: &'b mut Enumerate<Iter<'_, AccountInfo<'a>>>,
        internal_mining_type: Option<MiningType>,
    ) -> Result<Solend<'a>, ProgramError> {
        let reserve_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let reserve_liquidity_supply_info =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let lending_market_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let lending_market_authority_info = AccountLoader::next_unchecked(account_info_iter)?;
        let reserve_liquidity_pyth_oracle_info = AccountLoader::next_unchecked(account_info_iter)?;
        let reserve_liquidity_switchboard_oracle_info =
            AccountLoader::next_unchecked(account_info_iter)?;

        let mut solend = Solend {
            money_market_program_id,
            reserve: reserve_info.clone(),
            reserve_liquidity_supply: reserve_liquidity_supply_info.clone(),
            lending_market: lending_market_info.clone(),
            lending_market_authority: lending_market_authority_info.clone(),
            reserve_liquidity_pyth_oracle: reserve_liquidity_pyth_oracle_info.clone(),
            reserve_liquidity_switchboard_oracle: reserve_liquidity_switchboard_oracle_info.clone(),

            mining: None,
        };

        match internal_mining_type {
            Some(MiningType::Solend { obligation }) => {
                let obligation_info = AccountLoader::next_unchecked(account_info_iter)?;
                assert_account_key(obligation_info, &obligation)?;

                let collateral_supply_info =
                    AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

                solend.mining = Some(SolendMining {
                    obligation_info: obligation_info.clone(),
                    collateral_supply_info: collateral_supply_info.clone(),
                })
            }
            _ => {}
        }

        Ok(solend)
    }
}

impl<'a> MoneyMarket<'a> for Solend<'a> {
    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'a>,
        source_liquidity: AccountInfo<'a>,
        destination_collateral: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        solend::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_pyth_oracle.clone(),
            self.reserve_liquidity_switchboard_oracle.clone(),
            clock.clone(),
        )?;

        solend::deposit(
            &self.money_market_program_id,
            source_liquidity,
            destination_collateral.clone(),
            self.reserve.clone(),
            self.reserve_liquidity_supply.clone(),
            collateral_mint,
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            authority,
            clock,
            liquidity_amount,
            signers_seeds,
        )?;

        let collateral_amount =
            Account::unpack_unchecked(&destination_collateral.data.borrow())?.amount;

        Ok(collateral_amount)
    }

    fn money_market_redeem(
        &self,
        collateral_mint: AccountInfo<'a>,
        source_collateral: AccountInfo<'a>,
        destination_liquidity: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        solend::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_pyth_oracle.clone(),
            self.reserve_liquidity_switchboard_oracle.clone(),
            clock.clone(),
        )?;

        solend::redeem(
            &self.money_market_program_id,
            source_collateral,
            destination_liquidity,
            self.reserve.clone(),
            collateral_mint,
            self.reserve_liquidity_supply.clone(),
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            authority,
            clock,
            collateral_amount,
            signers_seeds,
        )
    }

    ///
    fn money_market_deposit_and_deposit_mining(
        &self,
        collateral_mint: AccountInfo<'a>,
        source_liquidity: AccountInfo<'a>,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
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

        if self.mining.is_some() {
            self.deposit_collateral_tokens(
                collateral_transit,
                authority,
                clock,
                collateral_amount,
                signers_seeds,
            )?
        } else {
            return Err(EverlendError::MiningNotInitialized.into());
        }

        Ok(collateral_amount)
    }

    ///
    fn money_market_redeem_and_withdraw_mining(
        &self,
        collateral_mint: AccountInfo<'a>,
        collateral_transit: AccountInfo<'a>,
        liquidity_destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        if self.mining.is_none() {
            return Err(EverlendError::MiningNotInitialized.into());
        }

        let mining = self.mining.as_ref().unwrap();

        solend::withdraw_obligation_collateral_and_redeem_reserve_collateral(
            &self.money_market_program_id,
            mining.collateral_supply_info.clone(),
            collateral_transit,
            self.reserve.clone(),
            self.reserve_liquidity_supply.clone(),
            collateral_mint,
            mining.obligation_info.clone(),
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            liquidity_destination,
            authority.clone(),
            authority.clone(),
            clock,
            collateral_amount,
            signers_seeds,
        )
    }
}

impl<'a> CollateralStorage<'a> for Solend<'a> {
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        if self.mining.is_none() {
            return Err(EverlendError::MiningNotInitialized.into());
        }

        solend::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_pyth_oracle.clone(),
            self.reserve_liquidity_switchboard_oracle.clone(),
            clock.clone(),
        )?;

        let mining = self.mining.as_ref().unwrap();

        solend::deposit_obligation_collateral(
            &self.money_market_program_id,
            collateral_transit.clone(),
            mining.collateral_supply_info.clone(),
            self.reserve.clone(),
            mining.obligation_info.clone(),
            self.lending_market.clone(),
            authority.clone(),
            authority.clone(),
            self.lending_market_authority.clone(),
            clock,
            collateral_amount,
            signers_seeds,
        )
    }

    fn withdraw_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        if self.mining.is_none() {
            return Err(EverlendError::MiningNotInitialized.into());
        }

        solend::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_pyth_oracle.clone(),
            self.reserve_liquidity_switchboard_oracle.clone(),
            clock.clone(),
        )?;

        let mining = self.mining.as_ref().unwrap();

        solend::refresh_obligation(
            &self.money_market_program_id,
            mining.obligation_info.clone(),
            self.reserve.clone(),
            clock.clone(),
        )?;

        solend::withdraw_obligation_collateral(
            &self.money_market_program_id,
            mining.collateral_supply_info.clone(),
            collateral_transit.clone(),
            self.reserve.clone(),
            mining.obligation_info.clone(),
            self.lending_market.clone(),
            authority,
            self.lending_market_authority.clone(),
            clock,
            collateral_amount,
            signers_seeds,
        )
    }
}
