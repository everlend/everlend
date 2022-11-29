use super::quarry::Quarry;
use super::{CollateralStorage, MoneyMarket};
use crate::state::MiningType;
use everlend_utils::{assert_account_key, cpi::port_finance, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

///
pub struct PortFinance<'a, 'b> {
    money_market_program_id: Pubkey,
    reserve: &'a AccountInfo<'b>,
    reserve_liquidity_supply: &'a AccountInfo<'b>,
    lending_market: &'a AccountInfo<'b>,
    lending_market_authority: &'a AccountInfo<'b>,
    reserve_liquidity_oracle: &'a AccountInfo<'b>,

    mining: Option<PortFinanceMining<'a, 'b>>,
    quarry_mining: Option<Quarry<'a, 'b>>,
}

///
struct PortFinanceMining<'a, 'b> {
    staking_program_id_info: &'a AccountInfo<'b>,
    staking_account_info: &'a AccountInfo<'b>,
    staking_pool_info: &'a AccountInfo<'b>,
    obligation_info: &'a AccountInfo<'b>,
    collateral_supply_pubkey_info: &'a AccountInfo<'b>,
}

impl<'a, 'b> PortFinance<'a, 'b> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        internal_mining_type: Option<MiningType>,
        collateral_token_mint: &Pubkey,
        depositor_authority: &Pubkey,
    ) -> Result<PortFinance<'a, 'b>, ProgramError> {
        let reserve_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let reserve_liquidity_supply_info =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let lending_market_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let lending_market_authority_info = AccountLoader::next_unchecked(account_info_iter)?;
        let reserve_liquidity_oracle_info = AccountLoader::next_unchecked(account_info_iter)?;

        let mut port_finance = PortFinance {
            money_market_program_id,
            reserve: reserve_info,
            reserve_liquidity_supply: reserve_liquidity_supply_info,
            lending_market: lending_market_info,
            lending_market_authority: lending_market_authority_info,
            reserve_liquidity_oracle: reserve_liquidity_oracle_info,

            mining: None,
            quarry_mining: None,
        };

        // Parse mining  accounts if presented
        match internal_mining_type {
            Some(MiningType::Quarry { rewarder }) => {
                let quarry = Quarry::init(
                    account_info_iter,
                    depositor_authority,
                    collateral_token_mint,
                    &rewarder,
                )?;

                port_finance.quarry_mining = Some(quarry)
            }
            Some(MiningType::PortFinance {
                staking_program_id,
                staking_account,
                staking_pool,
                obligation,
            }) => {
                let staking_program_id_info = AccountLoader::next_unchecked(account_info_iter)?;
                let staking_account_info =
                    AccountLoader::next_with_owner(account_info_iter, staking_program_id_info.key)?;
                let staking_pool_info =
                    AccountLoader::next_with_owner(account_info_iter, staking_program_id_info.key)?;

                assert_account_key(staking_program_id_info, &staking_program_id)?;
                assert_account_key(staking_account_info, &staking_account)?;
                assert_account_key(staking_pool_info, &staking_pool)?;

                let obligation_info = AccountLoader::next_unchecked(account_info_iter)?;
                assert_account_key(obligation_info, &obligation)?;

                let collateral_supply_pubkey_info =
                    AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

                port_finance.mining = Some(PortFinanceMining {
                    staking_program_id_info,
                    staking_account_info,
                    staking_pool_info,
                    obligation_info,
                    collateral_supply_pubkey_info,
                })
            }
            _ => {}
        }

        Ok(port_finance)
    }
}

impl<'a, 'b> MoneyMarket<'b> for PortFinance<'a, 'b> {
    fn is_collateral_return(&self) -> bool {
        true
    }

    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_liquidity: AccountInfo<'b>,
        destination_collateral: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        port_finance::deposit(
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
        collateral_mint: AccountInfo<'b>,
        source_collateral: AccountInfo<'b>,
        destination_liquidity: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        port_finance::redeem(
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

        // TODO use DepositReserveLiquidityAndObligationCollateral after fix of collateral leak
        if self.mining.is_some() {
            //Check collateral amount by obligation struct
            self.deposit_collateral_tokens(
                collateral_transit,
                authority,
                clock,
                collateral_amount,
                signers_seeds,
            )?
        } else if self.quarry_mining.is_some() {
            let quarry_mining = self.quarry_mining.as_ref().unwrap();
            quarry_mining.deposit_collateral_tokens(
                collateral_transit,
                authority,
                clock,
                collateral_amount,
                signers_seeds,
            )?
        } else {
            return Err(EverlendError::MiningNotInitialized.into());
        };

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
        if self.mining.is_some() {
            self.withdraw_collateral_tokens(
                collateral_transit.clone(),
                authority.clone(),
                clock.clone(),
                collateral_amount,
                signers_seeds,
            )?;
            self.refresh_reserve(clock.clone())?;
        } else if self.quarry_mining.is_some() {
            let quarry_mining = self.quarry_mining.as_ref().unwrap();
            quarry_mining.withdraw_collateral_tokens(
                collateral_transit.clone(),
                authority.clone(),
                clock.clone(),
                collateral_amount,
                signers_seeds,
            )?
        } else {
            return Err(EverlendError::MiningNotInitialized.into());
        };

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

    fn is_income(
        &self,
        collateral_amount: u64,
        expected_liquidity_amount: u64,
    ) -> Result<bool, ProgramError> {
        let real_liquidity_amount =
            port_finance::get_real_liquidity_amount(self.reserve.clone(), collateral_amount)?;

        Ok(real_liquidity_amount > expected_liquidity_amount)
    }

    fn refresh_reserve(&self, clock: AccountInfo<'b>) -> Result<(), ProgramError> {
        port_finance::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
            clock.clone(),
        )
    }

    fn is_deposit_disabled(&self) -> Result<bool, ProgramError> {
        // Not presented
        Ok(false)
    }
}

impl<'a, 'b> CollateralStorage<'b> for PortFinance<'a, 'b> {
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        if self.mining.is_none() {
            return Err(EverlendError::MiningNotInitialized.into());
        }

        self.refresh_reserve(clock.clone())?;

        let mining = self.mining.as_ref().unwrap();

        // Mining by obligation
        port_finance::deposit_obligation_collateral(
            &self.money_market_program_id,
            collateral_transit.clone(),
            mining.collateral_supply_pubkey_info.clone(),
            self.reserve.clone(),
            mining.obligation_info.clone(),
            self.lending_market.clone(),
            authority.clone(),
            authority.clone(),
            mining.staking_account_info.clone(),
            mining.staking_pool_info.clone(),
            mining.staking_program_id_info.clone(),
            self.lending_market_authority.clone(),
            clock,
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
            return Err(EverlendError::MiningNotInitialized.into());
        }

        let mining = self.mining.as_ref().unwrap();

        port_finance::refresh_obligation(
            &self.money_market_program_id,
            mining.obligation_info.clone(),
            self.reserve.clone(),
            clock.clone(),
        )?;

        // Mining by obligation
        port_finance::withdraw_obligation_collateral(
            &self.money_market_program_id,
            mining.collateral_supply_pubkey_info.clone(),
            collateral_transit,
            self.reserve.clone(),
            mining.obligation_info.clone(),
            self.lending_market.clone(),
            authority,
            mining.staking_account_info.clone(),
            mining.staking_pool_info.clone(),
            mining.staking_program_id_info.clone(),
            self.lending_market_authority.clone(),
            clock,
            collateral_amount,
            signers_seeds,
        )
    }
}
