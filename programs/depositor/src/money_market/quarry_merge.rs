use super::CollateralStorage;
use crate::money_market::MoneyMarket;
use crate::state::MiningType;
use everlend_utils::cpi::quarry_merge;
use everlend_utils::{
    cpi::{quarry, spl_token},
    AccountLoader, EverlendError,
};
use solana_program::program_pack::Pack;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;
use ::spl_token::{state::Account};
use std::{iter::Enumerate, slice::Iter};

///
#[derive(Clone)]
pub struct QuarryMerge<'a, 'b> {
    quarry_merge_mining_program_id: Pubkey,
    mm_primary_token_account: &'a AccountInfo<'b>,
    primary_token_mint: &'a AccountInfo<'b>,
    replica_mint: &'a AccountInfo<'b>,
    replica_mint_token_account: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    merge_miner: &'a AccountInfo<'b>,
    rewarder: &'a AccountInfo<'b>,
    quarry: &'a AccountInfo<'b>,
    miner: &'a AccountInfo<'b>,
    miner_vault: &'a AccountInfo<'b>,
}

impl<'a, 'b> QuarryMerge<'a, 'b> {
    ///
    pub fn init(
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        depositor_authority_pubkey: &Pubkey,
        token_mint: &'a AccountInfo<'b>,
        internal_mining_type: Option<MiningType>,
    ) -> Result<QuarryMerge<'a, 'b>, ProgramError> {
        if internal_mining_type.is_none() {
            return Err(EverlendError::MiningNotInitialized.into());
        }

        let (pool, rewarder) = match internal_mining_type {
            Some(MiningType::QuarryMerge { pool, rewarder }) => (pool, rewarder),
            _ => return Err(EverlendError::MiningNotInitialized.into()),
        };

        let quarry_merge_mining_program_id_info =
            AccountLoader::next_with_key(account_info_iter, &quarry_merge::staking_program_id())?;
        let mm_primary_token_account = AccountLoader::next_with_owner(account_info_iter, &::spl_token::id())?;
        let replica_mint = {
            let (replica_mint_pubkey, _) = quarry_merge::find_replica_mint_program_address(
                &quarry_merge::staking_program_id(),
                &pool,
            );
            AccountLoader::next_with_key(account_info_iter, &replica_mint_pubkey)
        }?;
        let replica_mint_token_account = AccountLoader::next_with_owner(account_info_iter, &::spl_token::id())?;
        let pool = AccountLoader::next_with_key(account_info_iter, &pool)?;

        let merge_miner = {
            let (merge_miner_pubkey, _) = quarry_merge::find_merge_miner_program_address(
                &quarry_merge::staking_program_id(),
                pool.key,
                depositor_authority_pubkey,
            );
            AccountLoader::next_with_key(account_info_iter, &merge_miner_pubkey)
        }?;
        let rewarder = AccountLoader::next_with_key(account_info_iter, &rewarder)?;
        let quarry = {
            let (quarry, _) = quarry::find_quarry_program_address(
                &quarry::staking_program_id(),
                rewarder.key,
                token_mint.key,
            );
            AccountLoader::next_with_key(account_info_iter, &quarry)
        }?;
        let miner = {
            let (miner_pubkey, _) = quarry::find_miner_program_address(
                &quarry::staking_program_id(),
                quarry.key,
                merge_miner.key,
            );
            AccountLoader::next_with_key(account_info_iter, &miner_pubkey)
        }?;
        let miner_vault = {
            let miner_vault = get_associated_token_address(miner.key, token_mint.key);
            AccountLoader::next_with_key(account_info_iter, &miner_vault)
        }?;

        Ok(QuarryMerge {
            quarry_merge_mining_program_id: *quarry_merge_mining_program_id_info.key,
            mm_primary_token_account,
            primary_token_mint: token_mint,
            replica_mint,
            replica_mint_token_account,
            pool,
            merge_miner,
            rewarder,
            quarry,
            miner,
            miner_vault,
        })
    }
}

impl<'a, 'b> MoneyMarket<'b> for QuarryMerge<'a, 'b> {
    ///
    fn money_market_deposit(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _source_liquidity: AccountInfo<'b>,
        _destination_collateral: AccountInfo<'b>,
        _authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        return Err(EverlendError::MiningIsRequired.into());
    }

    ///
    fn money_market_redeem(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _source_collateral: AccountInfo<'b>,
        _destination_liquidity: AccountInfo<'b>,
        _authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        _amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        return Err(EverlendError::MiningIsRequired.into());
    }

    ///
    fn money_market_deposit_and_deposit_mining(
        &self,
        _collateral_mint: AccountInfo<'b>,
        _source_liquidity: AccountInfo<'b>,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        clock: AccountInfo<'b>,
        _liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
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
        _collateral_mint: AccountInfo<'b>,
        collateral_transit: AccountInfo<'b>,
        _liquidity_destination: AccountInfo<'b>,
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
        )
    }
}

impl<'a, 'b> CollateralStorage<'b> for QuarryMerge<'a, 'b> {
    /// Deposit collateral tokens
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
       spl_token::transfer(
            collateral_transit.clone(),
            self.mm_primary_token_account.clone(),
            authority.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        quarry_merge::stake_primary(
            &self.quarry_merge_mining_program_id,
            authority.clone(),
            self.mm_primary_token_account.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.rewarder.clone(),
            self.quarry.clone(),
            self.miner.clone(),
            self.miner_vault.clone(),
            signers_seeds,
        )?;

        quarry_merge::stake_replica(
            &self.quarry_merge_mining_program_id,
            authority.clone(),
            self.replica_mint.clone(),
            self.replica_mint_token_account.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.rewarder.clone(),
            self.quarry.clone(),
            self.miner.clone(),
            self.miner_vault.clone(),
            signers_seeds,
        )?;

        Ok(())
    }
    /// Withdraw collateral tokens
    fn withdraw_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        quarry_merge::unstake_replica(
            &self.quarry_merge_mining_program_id,
            authority.clone(),
            self.replica_mint.clone(),
            self.replica_mint_token_account.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.rewarder.clone(),
            self.quarry.clone(),
            self.miner.clone(),
            self.miner_vault.clone(),
            signers_seeds,
        )?;

        quarry_merge::unstake_primary(
            &self.quarry_merge_mining_program_id,
            authority.clone(),
            self.mm_primary_token_account.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.rewarder.clone(),
            self.quarry.clone(),
            self.miner.clone(),
            self.miner_vault.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        quarry_merge::withdraw_tokens(
            &self.quarry_merge_mining_program_id,
            authority.clone(),
            self.pool.clone(),
            self.merge_miner.clone(),
            self.primary_token_mint.clone(),
            self.mm_primary_token_account.clone(),
            collateral_transit.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        if Account::unpack_from_slice(self.mm_primary_token_account.data.borrow().as_ref())?.amount
            != 0
        {
            quarry_merge::stake_replica(
                &self.quarry_merge_mining_program_id,
                authority.clone(),
                self.replica_mint.clone(),
                self.replica_mint_token_account.clone(),
                self.pool.clone(),
                self.merge_miner.clone(),
                self.rewarder.clone(),
                self.quarry.clone(),
                self.miner.clone(),
                self.miner_vault.clone(),
                signers_seeds,
            )?;
        }

        Ok(())
    }
}
