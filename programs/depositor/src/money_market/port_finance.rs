use super::{CollateralStorage, MoneyMarket};
use crate::state::MiningType;
use everlend_utils::{assert_account_key, cpi::port_finance, EverlendError};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Account;
use std::slice::Iter;

///
pub struct PortFinance<'a> {
    money_market_program_id: Pubkey,
    reserve: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    reserve_liquidity_oracle: AccountInfo<'a>,

    mining: Option<PortFinanceMining<'a>>,
}

///
struct PortFinanceMining<'a> {
    staking_program_id_info: AccountInfo<'a>,
    staking_account_info: AccountInfo<'a>,
    staking_pool_info: AccountInfo<'a>,
    obligation_info: AccountInfo<'a>,
    collateral_supply_pubkey_info: AccountInfo<'a>,
}

impl<'a> PortFinance<'a> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        account_info_iter: &mut Iter<AccountInfo<'a>>,
        internal_mining_type: Option<MiningType>,
    ) -> Result<PortFinance<'a>, ProgramError> {
        let reserve_info = next_account_info(account_info_iter)?;
        let reserve_liquidity_supply_info = next_account_info(account_info_iter)?;
        let lending_market_info = next_account_info(account_info_iter)?;
        let lending_market_authority_info = next_account_info(account_info_iter)?;
        let reserve_liquidity_oracle_info = next_account_info(account_info_iter)?;

        let mut port_finance = PortFinance {
            money_market_program_id,
            reserve: reserve_info.clone(),
            reserve_liquidity_supply: reserve_liquidity_supply_info.clone(),
            lending_market: lending_market_info.clone(),
            lending_market_authority: lending_market_authority_info.clone(),
            reserve_liquidity_oracle: reserve_liquidity_oracle_info.clone(),

            mining: None,
        };

        // Parse mining  accounts if presented
        match internal_mining_type {
            Some(MiningType::PortFinance {
                staking_program_id,
                staking_account,
                staking_pool,
                obligation,
            }) => {
                let staking_program_id_info = next_account_info(account_info_iter)?;
                let staking_account_info = next_account_info(account_info_iter)?;
                let staking_pool_info = next_account_info(account_info_iter)?;

                assert_account_key(staking_program_id_info, &staking_program_id)?;
                assert_account_key(staking_account_info, &staking_account)?;
                assert_account_key(staking_pool_info, &staking_pool)?;

                let obligation_info = next_account_info(account_info_iter)?;
                assert_account_key(obligation_info, &obligation)?;

                let collateral_supply_pubkey_info = next_account_info(account_info_iter)?;

                port_finance.mining = Some(PortFinanceMining {
                    staking_program_id_info: staking_program_id_info.clone(),
                    staking_account_info: staking_account_info.clone(),
                    staking_pool_info: staking_pool_info.clone(),
                    obligation_info: obligation_info.clone(),
                    collateral_supply_pubkey_info: collateral_supply_pubkey_info.clone(),
                });
            }
            _ => {}
        }

        Ok(port_finance)
    }
}

impl<'a> MoneyMarket<'a> for PortFinance<'a> {
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
        port_finance::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
            clock.clone(),
        )?;

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
        collateral_mint: AccountInfo<'a>,
        source_collateral: AccountInfo<'a>,
        destination_liquidity: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        port_finance::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
            clock.clone(),
        )?;

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

        // TODO use DepositReserveLiquidityAndObligationCollateral after fix of collateral leak
        //Check collateral amount by obligation struct
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
        collateral_mint: AccountInfo<'a>,
        collateral_transit: AccountInfo<'a>,
        liquidity_destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
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

impl<'a> CollateralStorage<'a> for PortFinance<'a> {
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

        port_finance::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
            clock.clone(),
        )?;

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
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        if self.mining.is_none() {
            return Err(EverlendError::MiningNotInitialized.into());
        }

        port_finance::refresh_reserve(
            &self.money_market_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
            clock.clone(),
        )?;

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
