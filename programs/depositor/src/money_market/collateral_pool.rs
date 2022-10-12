use super::CollateralStorage;
use everlend_collateral_pool::{
    cpi, find_pool_withdraw_authority_program_address, utils::CollateralPoolAccounts,
};
use everlend_registry::state::RegistryMarkets;
use everlend_utils::{assert_account_key, AccountLoader};
use solana_program::{
    account_info::AccountInfo, msg, program_error::ProgramError, program_pack::Pack,
};
use std::{iter::Enumerate, slice::Iter};

/// Container
#[derive(Clone)]
pub struct CollateralPool<'a, 'b> {
    collateral_pool_market: &'a AccountInfo<'b>,
    collateral_pool_market_authority: &'a AccountInfo<'b>,
    collateral_pool: &'a AccountInfo<'b>,
    collateral_pool_token_account: &'a AccountInfo<'b>,
    collateral_pool_withdraw_authority: Option<&'a AccountInfo<'b>>,
}

impl<'a, 'b> CollateralPool<'a, 'b> {
    ///
    pub fn init(
        registry_markets: &RegistryMarkets,
        collateral_mint: &AccountInfo<'b>,
        authority: &AccountInfo<'b>,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        is_withdraw_expected: bool,
    ) -> Result<CollateralPool<'a, 'b>, ProgramError> {
        msg!("Init CollateralPool");
        let collateral_pool_market_info =
            AccountLoader::next_with_owner(account_info_iter, &everlend_collateral_pool::id())?;
        let collateral_pool_market_authority_info =
            AccountLoader::next_unchecked(account_info_iter)?;
        let collateral_pool_info =
            AccountLoader::next_with_owner(account_info_iter, &everlend_collateral_pool::id())?;
        let collateral_pool_token_account_info =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        // Check collateral pool market
        if !registry_markets
            .collateral_pool_markets
            .contains(collateral_pool_market_info.key)
        {
            return Err(ProgramError::InvalidArgument);
        }

        {
            // Check collateral pool
            let (collateral_pool_pubkey, _) = everlend_collateral_pool::find_pool_program_address(
                &everlend_collateral_pool::id(),
                collateral_pool_market_info.key,
                collateral_mint.key,
            );
            assert_account_key(collateral_pool_info, &collateral_pool_pubkey)?;
        }

        {
            let collateral_pool =
                everlend_collateral_pool::state::Pool::unpack(&collateral_pool_info.data.borrow())?;

            // Check collateral pool accounts
            assert_account_key(collateral_mint, &collateral_pool.token_mint)?;
            assert_account_key(
                collateral_pool_token_account_info,
                &collateral_pool.token_account,
            )?;
        }

        let _everlend_collateral_pool_info =
            AccountLoader::next_with_key(account_info_iter, &everlend_collateral_pool::id())?;

        let mut collateral_pool_withdraw_authority: Option<&'a AccountInfo<'b>> = None;
        if is_withdraw_expected {
            let collateral_pool_withdraw_authority_info =
                AccountLoader::next_unchecked(account_info_iter)?;

            let (collateral_pool_withdraw_authority_pubkey, _) =
                find_pool_withdraw_authority_program_address(
                    &everlend_collateral_pool::id(),
                    collateral_pool_info.key,
                    authority.key,
                );

            assert_account_key(
                collateral_pool_withdraw_authority_info,
                &collateral_pool_withdraw_authority_pubkey,
            )?;

            collateral_pool_withdraw_authority = Some(collateral_pool_withdraw_authority_info);
        }

        Ok(CollateralPool {
            collateral_pool_market: collateral_pool_market_info,
            collateral_pool_market_authority: collateral_pool_market_authority_info,
            collateral_pool: collateral_pool_info,
            collateral_pool_token_account: collateral_pool_token_account_info,
            collateral_pool_withdraw_authority,
        })
    }
}

impl<'a, 'b> CollateralStorage<'b> for CollateralPool<'a, 'b> {
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        msg!("Collect collateral tokens to Collateral Pool");

        cpi::deposit(
            CollateralPoolAccounts {
                pool_market: self.collateral_pool_market.clone(),
                pool_market_authority: self.collateral_pool_market_authority.clone(),
                pool: self.collateral_pool.clone(),
                token_account: self.collateral_pool_token_account.clone(),
            },
            collateral_transit,
            authority,
            collateral_amount,
            signers_seeds,
        )
    }

    fn withdraw_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'b>,
        authority: AccountInfo<'b>,
        _clock: AccountInfo<'b>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        msg!("Withdraw collateral tokens from Collateral Pool");
        if self.collateral_pool_withdraw_authority.is_none() {
            return Err(ProgramError::InvalidArgument);
        }

        cpi::withdraw(
            CollateralPoolAccounts {
                pool_market: self.collateral_pool_market.clone(),
                pool_market_authority: self.collateral_pool_market_authority.clone(),
                pool: self.collateral_pool.clone(),
                token_account: self.collateral_pool_token_account.clone(),
            },
            self.collateral_pool_withdraw_authority.unwrap().clone(),
            collateral_transit,
            authority,
            collateral_amount,
            signers_seeds,
        )
    }
}
