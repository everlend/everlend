use crate::{
    find_internal_mining_program_address,
    state::{Depositor, InternalMining, MiningType},
};

use everlend_registry::state::Registry;
use everlend_utils::{
    assert_account_key, assert_owned_by, cpi, find_program_address, AccountLoader, EverlendError,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey, rent::Rent, system_program, sysvar::clock, sysvar::Sysvar,
    sysvar::SysvarId,
};
use spl_associated_token_account::get_associated_token_address;
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct InitMiningAccountContext<'a, 'b> {
    internal_mining: &'a AccountInfo<'b>,
    liquidity_mint: &'a AccountInfo<'b>,
    collateral_mint: &'a AccountInfo<'b>,
    depositor: &'a AccountInfo<'b>,
    depositor_authority: &'a AccountInfo<'b>,
    registry: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitMiningAccountContext<'a, 'b> {
    /// New InitMiningAccount instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<InitMiningAccountContext<'a, 'b>, ProgramError> {
        let internal_mining = AccountLoader::next_optional(account_info_iter, &program_id)?;
        let liquidity_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let collateral_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let depositor = AccountLoader::next_with_owner(account_info_iter, &program_id)?;
        let depositor_authority = AccountLoader::next_unchecked(account_info_iter)?;
        let registry = AccountLoader::next_with_owner(account_info_iter, &everlend_registry::id())?;
        let manager = AccountLoader::next_signer(account_info_iter)?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;

        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        Ok(InitMiningAccountContext {
            internal_mining,
            liquidity_mint,
            collateral_mint,
            depositor,
            depositor_authority,
            registry,
            manager,
            rent,
        })
    }

    /// Process InitMiningAccount instruction
    pub fn process(
        &self,
        program_id: &Pubkey,
        account_info_iter: &'a mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        mining_type: MiningType,
    ) -> ProgramResult {
        {
            let depositor = Depositor::unpack(&self.depositor.data.borrow())?;
            assert_account_key(self.registry, &depositor.registry)?;
        }

        {
            let registry = Registry::unpack(&self.registry.data.borrow())?;
            assert_account_key(self.manager, &registry.manager)?;
        }

        let internal_mining_bump_seed = {
            let (internal_mining_pubkey, internal_mining_bump_seed) =
                find_internal_mining_program_address(
                    program_id,
                    self.liquidity_mint.key,
                    self.collateral_mint.key,
                    self.depositor.key,
                );
            assert_account_key(self.internal_mining, &internal_mining_pubkey)?;
            internal_mining_bump_seed
        };

        // Check depositor authority account
        let signers_seeds = {
            let (depositor_authority_pubkey, bump_seed) =
                find_program_address(program_id, self.depositor.key);
            assert_account_key(self.depositor_authority, &depositor_authority_pubkey)?;
            &[&self.depositor.key.to_bytes()[..32], &[bump_seed]]
        };

        let rent = &Rent::from_account_info(self.rent)?;

        // Create internal mining account
        if !self.internal_mining.owner.eq(program_id) {
            let signers_seeds = &[
                "internal_mining".as_bytes(),
                &self.liquidity_mint.key.to_bytes()[..32],
                &self.collateral_mint.key.to_bytes()[..32],
                &self.depositor.key.to_bytes()[..32],
                &[internal_mining_bump_seed],
            ];

            cpi::system::create_account::<InternalMining>(
                program_id,
                self.manager.clone(),
                self.internal_mining.clone(),
                &[signers_seeds],
                rent,
            )?;
        } else {
            assert_owned_by(self.internal_mining, program_id)?;
            // Check that account
            InternalMining::unpack(&self.internal_mining.data.borrow())?;
        }

        let staking_program_id_info = AccountLoader::next_unchecked(account_info_iter)?;

        match mining_type {
            MiningType::Larix {
                mining_account,
                additional_reward_token_account,
            } => {
                let mining_account_info =
                    AccountLoader::next_with_key(account_info_iter, &mining_account)?;
                assert_owned_by(mining_account_info, staking_program_id_info.key)?;

                let lending_market_info = AccountLoader::next_unchecked(account_info_iter)?;
                if let Some(additional_reward_token_account) = additional_reward_token_account {
                    let additional_reward_token_account_info = AccountLoader::next_with_key(
                        account_info_iter,
                        &additional_reward_token_account,
                    )?;
                    assert_owned_by(additional_reward_token_account_info, &spl_token::id())?;

                    let token_account = spl_token::state::Account::unpack(
                        &additional_reward_token_account_info.data.borrow(),
                    )?;

                    let (depositor_authority_pubkey, _) =
                        find_program_address(program_id, self.depositor.key);
                    if !token_account.owner.eq(&depositor_authority_pubkey) {
                        return Err(EverlendError::InvalidAccountOwner.into());
                    }
                }

                cpi::larix::init_mining(
                    staking_program_id_info.key,
                    mining_account_info.clone(),
                    self.depositor_authority.clone(),
                    lending_market_info.clone(),
                    &[signers_seeds.as_ref()],
                )?
            }
            MiningType::PortFinance {
                staking_program_id,
                staking_account,
                staking_pool,
                obligation,
            } => {
                assert_account_key(staking_program_id_info, &staking_program_id)?;
                let staking_pool_info =
                    AccountLoader::next_with_key(account_info_iter, &staking_pool)?;
                let staking_account_info =
                    AccountLoader::next_with_key(account_info_iter, &staking_account)?;

                assert_owned_by(staking_account_info, staking_program_id_info.key)?;

                let money_market_program_id_info =
                    AccountLoader::next_unchecked(account_info_iter)?;
                let obligation_info = AccountLoader::next_with_key(account_info_iter, &obligation)?;

                let lending_market_info = AccountLoader::next_unchecked(account_info_iter)?;

                let clock = AccountLoader::next_with_key(account_info_iter, &clock::id())?;
                let _spl_token_program =
                    AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;

                cpi::port_finance::init_obligation(
                    money_market_program_id_info.key,
                    obligation_info.clone(),
                    lending_market_info.clone(),
                    self.depositor_authority.clone(),
                    clock.clone(),
                    self.rent.clone(),
                    &[signers_seeds.as_ref()],
                )?;

                cpi::port_finance::create_stake_account(
                    staking_program_id_info.key,
                    staking_account_info.clone(),
                    staking_pool_info.clone(),
                    self.depositor_authority.clone(),
                    self.rent.clone(),
                )?;
            }
            MiningType::Quarry { rewarder } => {
                assert_account_key(staking_program_id_info, &cpi::quarry::staking_program_id())?;

                let rewarder_info = AccountLoader::next_with_key(account_info_iter, &rewarder)?;

                let quarry_info = {
                    let (quarry, _) = cpi::quarry::find_quarry_program_address(
                        &cpi::quarry::staking_program_id(),
                        &rewarder,
                        self.collateral_mint.key,
                    );

                    AccountLoader::next_with_key(account_info_iter, &quarry)
                }?;

                let miner_info = {
                    let (miner_pubkey, _) = cpi::quarry::find_miner_program_address(
                        &cpi::quarry::staking_program_id(),
                        quarry_info.key,
                        self.depositor_authority.key,
                    );
                    AccountLoader::next_with_key(account_info_iter, &miner_pubkey)
                }?;

                let miner_vault_info = {
                    let miner_vault =
                        get_associated_token_address(miner_info.key, self.collateral_mint.key);
                    AccountLoader::next_with_key(account_info_iter, &miner_vault)
                }?;

                let _spl_token_program =
                    AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;

                cpi::quarry::create_miner(
                    staking_program_id_info.key,
                    self.depositor_authority.clone(),
                    miner_info.clone(),
                    quarry_info.clone(),
                    rewarder_info.clone(),
                    self.manager.clone(),
                    self.collateral_mint.clone(),
                    miner_vault_info.clone(),
                    &[signers_seeds.as_ref()],
                )?;
            }
            MiningType::None => {}
        }

        let mut internal_mining =
            InternalMining::unpack_unchecked(&self.internal_mining.data.borrow())?;
        internal_mining.init(mining_type);

        InternalMining::pack(internal_mining, *self.internal_mining.data.borrow_mut())?;

        Ok(())
    }
}