use super::CollateralStorage;
use everlend_collateral_pool::{cpi, find_pool_withdraw_authority_program_address, utils::CollateralPoolAccounts};
use everlend_registry::state::{RegistryPrograms, RegistryRootAccounts};
use everlend_utils::{assert_account_key, assert_owned_by};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    msg,
    program_error::ProgramError,
    program_pack::Pack,
};

/// Container
#[derive(Clone)]
pub struct CollateralPool<'a> {
    collateral_pool_market: AccountInfo<'a>,
    collateral_pool_market_authority: AccountInfo<'a>,
    collateral_pool: AccountInfo<'a>,
    collateral_pool_token_account: AccountInfo<'a>,
    collateral_pool_withdraw_authority: Option<AccountInfo<'a>>,
}

use std::slice::Iter;

impl<'a> CollateralPool<'a> {
    ///
    pub fn init(
        registry_programs: &RegistryPrograms,
        root_accounts: &RegistryRootAccounts,
        collateral_mint: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        account_info_iter: &mut Iter<AccountInfo<'a>>,
        is_withdraw_expected: bool,
    ) -> Result<CollateralPool<'a>, ProgramError> {
        let collateral_pool_market_info = next_account_info(account_info_iter)?;
        let collateral_pool_market_authority_info = next_account_info(account_info_iter)?;
        let collateral_pool_info = next_account_info(account_info_iter)?;
        let collateral_pool_token_account_info = next_account_info(account_info_iter)?;

        // Check external programs
        assert_owned_by(
            collateral_pool_market_info,
            &registry_programs.collateral_pool_program_id,
        )?;
        assert_owned_by(
            collateral_pool_info,
            &registry_programs.collateral_pool_program_id,
        )?;

        // Check collateral pool market
        if !root_accounts
            .collateral_pool_markets
            .contains(collateral_pool_market_info.key)
        {
            return Err(ProgramError::InvalidArgument);
        }

        // Check collateral pool
        let (collateral_pool_pubkey, _) = everlend_collateral_pool::find_pool_program_address(
            &registry_programs.collateral_pool_program_id,
            collateral_pool_market_info.key,
            collateral_mint.key,
        );
        assert_account_key(collateral_pool_info, &collateral_pool_pubkey)?;

        let collateral_pool =
            everlend_collateral_pool::state::Pool::unpack(&collateral_pool_info.data.borrow())?;

        // Check collateral pool accounts
        assert_account_key(&collateral_mint, &collateral_pool.token_mint)?;
        assert_account_key(
            collateral_pool_token_account_info,
            &collateral_pool.token_account,
        )?;

        let mut collateral_pool_withdraw_authority: Option<AccountInfo<'a>> = None;
        if is_withdraw_expected {
            let collateral_pool_withdraw_authority_info = next_account_info(account_info_iter)?;

            let (collateral_pool_withdraw_authority_pubkey, _) =
                find_pool_withdraw_authority_program_address(
                    &registry_programs.collateral_pool_program_id,
                    collateral_pool_info.key,
                    authority.key,
                );
            assert_account_key(
                collateral_pool_withdraw_authority_info,
                &collateral_pool_withdraw_authority_pubkey,
            )?;

            collateral_pool_withdraw_authority =
                Some(collateral_pool_withdraw_authority_info.clone());
        }

        let _everlend_collateral_pool_info = next_account_info(account_info_iter)?;

        Ok(CollateralPool {
            collateral_pool_market: collateral_pool_market_info.clone(),
            collateral_pool_market_authority: collateral_pool_market_authority_info.clone(),
            collateral_pool: collateral_pool_info.clone(),
            collateral_pool_token_account: collateral_pool_token_account_info.clone(),
            collateral_pool_withdraw_authority,
        })
    }
}

impl<'a> CollateralStorage<'a> for CollateralPool<'a> {
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
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
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
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
            self.collateral_pool_withdraw_authority
                .as_ref()
                .unwrap()
                .clone(),
            collateral_transit,
            authority,
            collateral_amount,
            signers_seeds,
        )
    }
}
