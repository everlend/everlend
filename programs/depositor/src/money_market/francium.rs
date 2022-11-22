use crate::find_transit_program_address;
use crate::money_market::{CollateralStorage, MoneyMarket};
use crate::state::MiningType;
use everlend_utils::cpi::francium;
use everlend_utils::{assert_account_key, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

///
pub struct Francium<'a, 'b> {
    money_market_program_id: Pubkey,
    reserve: &'a AccountInfo<'b>,
    reserve_liquidity_supply: &'a AccountInfo<'b>,
    lending_market: &'a AccountInfo<'b>,
    lending_market_authority: &'a AccountInfo<'b>,

    mining: Option<FranciumFarming<'a, 'b>>,
}

struct FranciumFarming<'a, 'b> {
    lend_reward_program_id: &'a AccountInfo<'b>,
    user_farming: &'a AccountInfo<'b>,
    user_reward_a: &'a AccountInfo<'b>,
    user_reward_b: &'a AccountInfo<'b>,
    farming_pool: &'a AccountInfo<'b>,
    farming_pool_authority: &'a AccountInfo<'b>,
    pool_stake_token: &'a AccountInfo<'b>,
    pool_reward_a: &'a AccountInfo<'b>,
    pool_reward_b: &'a AccountInfo<'b>,
}

impl<'a, 'b> Francium<'a, 'b> {
    ///
    pub fn init(
        program_id: &Pubkey,
        money_market_program_id: Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        depositor: &Pubkey,
        depositor_authority: &Pubkey,
        internal_mining_type: Option<MiningType>,
    ) -> Result<Francium<'a, 'b>, ProgramError> {
        let reserve_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let reserve_liquidity_supply_info =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let lending_market_info =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let lending_market_authority_info = AccountLoader::next_unchecked(account_info_iter)?;

        let mut francium = Francium {
            money_market_program_id,
            reserve: &reserve_info,
            reserve_liquidity_supply: &reserve_liquidity_supply_info,
            lending_market: &lending_market_info,
            lending_market_authority: &lending_market_authority_info,

            mining: None,
        };

        // Parse mining  accounts if presented
        match internal_mining_type {
            Some(MiningType::Francium {
                user_reward_a,
                user_reward_b,
                user_stake_token_account,
                farming_pool,
            }) => {
                let lend_reward_program_id_info = AccountLoader::next_with_key(
                    account_info_iter,
                    &francium::get_staking_program_id(),
                )?;
                let farming_pool_info =
                    AccountLoader::next_with_key(account_info_iter, &farming_pool)?;
                let farming_pool_authority_info = AccountLoader::next_unchecked(account_info_iter)?;

                let user_farming = francium::find_user_farming_address(
                    depositor_authority,
                    farming_pool_info.key,
                    &user_stake_token_account,
                );

                let user_farming_info =
                    AccountLoader::next_with_key(account_info_iter, &user_farming)?;
                let user_reward_a_info =
                    AccountLoader::next_with_key(account_info_iter, &user_reward_a)?;
                let user_reward_b_info =
                    AccountLoader::next_with_key(account_info_iter, &user_reward_b)?;
                let pool_stake_token_info =
                    AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
                let pool_reward_a_info =
                    AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
                let pool_reward_b_info =
                    AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
                let token_mint_address_a_info =
                    AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
                let token_mint_address_b_info =
                    AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

                let (user_reward_a_check, _) = find_transit_program_address(
                    program_id,
                    &depositor,
                    &token_mint_address_a_info.key,
                    francium::FRANCIUM_REWARD_SEED,
                );

                assert_account_key(&user_reward_a_info, &user_reward_a_check)?;

                let (user_reward_b_check, _) = find_transit_program_address(
                    program_id,
                    &depositor,
                    &token_mint_address_b_info.key,
                    francium::FRANCIUM_REWARD_SEED,
                );

                assert_account_key(&user_reward_b_info, &user_reward_b_check)?;

                francium.mining = Some(FranciumFarming {
                    lend_reward_program_id: &lend_reward_program_id_info,
                    user_farming: &user_farming_info,
                    user_reward_a: &user_reward_a_info,
                    user_reward_b: &user_reward_b_info,
                    farming_pool: &farming_pool_info,
                    farming_pool_authority: &farming_pool_authority_info,
                    pool_stake_token: &pool_stake_token_info,
                    pool_reward_a: &pool_reward_a_info,
                    pool_reward_b: &pool_reward_b_info,
                })
            }
            _ => {}
        }

        Ok(francium)
    }
}

impl<'a, 'b> MoneyMarket<'b> for Francium<'a, 'b> {
    ///
    fn is_collateral_return(&self) -> bool {
        true
    }

    ///
    fn money_market_deposit(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_liquidity: AccountInfo<'b>,
        destination_collateral: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        francium::refresh_reserve(&self.money_market_program_id, self.reserve.clone())?;

        francium::deposit(
            &self.money_market_program_id,
            source_liquidity,
            destination_collateral.clone(),
            self.reserve.clone(),
            collateral_mint,
            self.reserve_liquidity_supply.clone(),
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            authority,
            clock,
            amount,
            signers_seeds,
        )?;

        let collateral_amount =
            Account::unpack_from_slice(&destination_collateral.data.borrow())?.amount;

        Ok(collateral_amount)
    }

    ///
    fn money_market_redeem(
        &self,
        collateral_mint: AccountInfo<'b>,
        source_collateral: AccountInfo<'b>,
        destination_liquidity: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        francium::refresh_reserve(&self.money_market_program_id, self.reserve.clone())?;

        francium::redeem(
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
            amount,
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
}

impl<'a, 'b> CollateralStorage<'b> for Francium<'a, 'b> {
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
        let mining = self.mining.as_ref().unwrap();

        francium::stake(
            &mining.lend_reward_program_id.key,
            authority.clone(),
            mining.user_farming.clone(),
            collateral_transit.clone(),
            mining.user_reward_a.clone(),
            mining.user_reward_b.clone(),
            mining.farming_pool.clone(),
            mining.farming_pool_authority.clone(),
            mining.pool_stake_token.clone(),
            mining.pool_reward_a.clone(),
            mining.pool_reward_b.clone(),
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

        francium::unstake(
            &mining.lend_reward_program_id.key,
            authority.clone(),
            mining.user_farming.clone(),
            collateral_transit.clone(),
            mining.user_reward_a.clone(),
            mining.user_reward_b.clone(),
            mining.farming_pool.clone(),
            mining.farming_pool_authority.clone(),
            mining.pool_stake_token.clone(),
            mining.pool_reward_a.clone(),
            mining.pool_reward_b.clone(),
            clock,
            collateral_amount,
            signers_seeds,
        )
    }
}
