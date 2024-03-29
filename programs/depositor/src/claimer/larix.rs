use crate::claimer::RewardClaimer;
use crate::state::MiningType;
use crate::utils::FillRewardAccounts;
use everlend_utils::cpi::larix;
use everlend_utils::{assert_account_key, find_program_address, AccountLoader, EverlendError};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use std::{iter::Enumerate, slice::Iter};
use solana_program::program_pack::Pack;

/// Container
#[derive(Clone)]
pub struct LarixClaimer<'a, 'b> {
    lending_market: &'a AccountInfo<'b>,
    lending_market_authority: &'a AccountInfo<'b>,
    reserve: &'a AccountInfo<'b>,
    reserve_liquidity_oracle: &'a AccountInfo<'b>,
    mining_account: &'a AccountInfo<'b>,
    mine_supply: &'a AccountInfo<'b>,
}

impl<'a, 'b> LarixClaimer<'a, 'b> {
    ///
    pub fn init(
        staking_program_id: &Pubkey,
        internal_mining_type: MiningType,
        with_subrewards: bool,
        fill_sub_rewards_accounts: Option<FillRewardAccounts<'a, 'b>>,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<LarixClaimer<'a, 'b>, ProgramError> {
        // Parse mining  accounts if presented
        let mining_account_pubkey = match internal_mining_type {
            MiningType::Larix {
                mining_account,
                additional_reward_token_account,
            } => {
                if with_subrewards != additional_reward_token_account.is_some() {
                    return Err(ProgramError::InvalidArgument);
                };

                if with_subrewards {
                    // Assert additional reward token account
                    assert_account_key(
                        &fill_sub_rewards_accounts.unwrap().reward_transit_info,
                        &additional_reward_token_account.unwrap(),
                    )?;
                };

                mining_account
            }
            _ => return Err(EverlendError::MiningNotInitialized.into()),
        };

        {
            let registry = AccountLoader::next_with_owner(account_info_iter, &everlend_registry::id())?;
            let registry_markets
                = everlend_registry::state::RegistryMarkets::unpack_from_slice(&registry.data.borrow())?;
            if !registry_markets.money_markets.contains(staking_program_id) {
                return Err(ProgramError::InvalidArgument);
            }
        }

        let mining_account =
            AccountLoader::next_with_key(account_info_iter, &mining_account_pubkey)?;

        let mine_supply = AccountLoader::next_unchecked(account_info_iter)?;
        let lending_market = AccountLoader::next_with_owner(account_info_iter, staking_program_id)?;
        let lending_market_authority = {
            let (lending_market_authority_pubkey, _) =
                find_program_address(staking_program_id, lending_market.key);
            AccountLoader::next_with_key(account_info_iter, &lending_market_authority_pubkey)?
        };
        let reserve = AccountLoader::next_with_owner(account_info_iter, staking_program_id)?;
        let reserve_liquidity_oracle = AccountLoader::next_unchecked(account_info_iter)?;

        Ok(LarixClaimer {
            mining_account,
            mine_supply,
            lending_market,
            lending_market_authority,
            reserve,
            reserve_liquidity_oracle,
        })
    }
}

impl<'a, 'b> RewardClaimer<'b> for LarixClaimer<'a, 'b> {
    ///
    fn claim_reward(
        &self,
        staking_program_id: &Pubkey,
        reward_transit_token_account: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        larix::refresh_mine(
            staking_program_id,
            self.mining_account.clone(),
            self.reserve.clone(),
        )?;

        larix::refresh_reserve(
            staking_program_id,
            self.reserve.clone(),
            self.reserve_liquidity_oracle.clone(),
        )?;

        larix::claim_mine(
            staking_program_id,
            self.mining_account.clone(),
            self.mine_supply.clone(),
            reward_transit_token_account,
            authority,
            self.lending_market.clone(),
            self.lending_market_authority.clone(),
            self.reserve.clone(),
            signers_seeds,
        )?;

        Ok(())
    }
}
