use crate::money_market::{CollateralStorage, MoneyMarket};
use crate::state::MiningType;
use everlend_utils::cpi::mango;
use everlend_utils::{AccountLoader, EverlendError};
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::iter::Enumerate;
use std::slice::Iter;

///
pub struct Mango<'a> {
    money_market_program_id: Pubkey,
    mango_group: AccountInfo<'a>,
    mango_cache: AccountInfo<'a>,
    root_bank: AccountInfo<'a>,
    node_bank: AccountInfo<'a>,
    vault: AccountInfo<'a>,
    mining: Option<MangoMining<'a>>,
}

struct MangoMining<'a> {
    mango_account: AccountInfo<'a>,
}

impl<'a, 'b> Mango<'a> {
    ///
    pub fn init(
        money_market_program_id: Pubkey,
        account_info_iter: &'b mut Enumerate<Iter<'_, AccountInfo<'a>>>,
        internal_mining_type: Option<MiningType>,
        depositor_authority_pubkey: &Pubkey,
    ) -> Result<Mango<'a>, ProgramError> {
        let mango_group =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let mango_cache =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let root_bank =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let node_bank =
            AccountLoader::next_with_owner(account_info_iter, &money_market_program_id)?;
        let vault = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let mut mango = Mango {
            money_market_program_id,
            mango_group: mango_group.clone(),
            mango_cache: mango_cache.clone(),
            root_bank: root_bank.clone(),
            node_bank: node_bank.clone(),
            vault: vault.clone(),
            mining: None,
        };

        match internal_mining_type {
            Some(MiningType::Mango {
                staking_program_id,
                mango_group,
            }) => {
                let mango_account = {
                    let (mango_account_pubkey, _) = mango::find_account_program_address(
                        &staking_program_id,
                        &mango_group,
                        depositor_authority_pubkey,
                    );

                    AccountLoader::next_with_key(account_info_iter, &mango_account_pubkey)?
                };

                mango.mining = Some(MangoMining {
                    mango_account: mango_account.clone(),
                })
            }
            _ => {}
        }

        Ok(mango)
    }
}

impl<'a> MoneyMarket<'a> for Mango<'a> {
    fn money_market_deposit(
        &self,
        _collateral_mint: AccountInfo<'a>,
        _source_liquidity: AccountInfo<'a>,
        _destination_collateral: AccountInfo<'a>,
        _authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
        _liquidity_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        return Err(EverlendError::MiningIsRequired.into());
    }

    fn money_market_redeem(
        &self,
        _collateral_mint: AccountInfo<'a>,
        _source_collateral: AccountInfo<'a>,
        _destination_liquidity: AccountInfo<'a>,
        _authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
        _collateral_amount: u64,
        _signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        return Err(EverlendError::MiningIsRequired.into());
    }

    fn money_market_deposit_and_deposit_mining(
        &self,
        _collateral_mint: AccountInfo<'a>,
        source_liquidity: AccountInfo<'a>,
        _collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        liquidity_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<u64, ProgramError> {
        self.deposit_collateral_tokens(
            source_liquidity,
            authority,
            clock,
            liquidity_amount,
            signers_seeds,
        )?;

        Ok(liquidity_amount)
    }

    fn money_market_redeem_and_withdraw_mining(
        &self,
        _collateral_mint: AccountInfo<'a>,
        _collateral_transit: AccountInfo<'a>,
        liquidity_destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        self.withdraw_collateral_tokens(
            liquidity_destination,
            authority,
            clock,
            collateral_amount,
            signers_seeds,
        )
    }
}

impl<'a> CollateralStorage<'a> for Mango<'a> {
    fn deposit_collateral_tokens(
        &self,
        collateral_transit: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        _clock: AccountInfo<'a>,
        collateral_amount: u64,
        signers_seeds: &[&[&[u8]]],
    ) -> Result<(), ProgramError> {
        mango::deposit(
            &self.money_market_program_id,
            self.mango_group.clone(),
            self.mining.as_ref().unwrap().mango_account.clone(),
            authority,
            self.mango_cache.clone(),
            self.root_bank.clone(),
            self.node_bank.clone(),
            self.vault.clone(),
            collateral_transit,
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
        mango::withdraw(
            &self.money_market_program_id,
            self.mango_group.clone(),
            self.mining.as_ref().unwrap().mango_account.clone(),
            authority,
            self.mango_cache.clone(),
            self.root_bank.clone(),
            self.node_bank.clone(),
            self.vault.clone(),
            collateral_transit,
            collateral_amount,
            signers_seeds,
        )
    }
}
