use crate::{
    find_internal_mining_program_address, find_transit_program_address,
    state::{Depositor, InternalMining, MiningType},
    InternalMiningPDA,
};

use borsh::BorshDeserialize;
use everlend_registry::state::Registry;
use everlend_utils::cpi::francium;
use everlend_utils::{
    assert_account_key, assert_owned_by, cpi, find_program_address, AccountLoader, EverlendError,
    PDA,
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
    staking_program_id: &'a AccountInfo<'b>,
    registry: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
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

        let system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        let staking_program_id = AccountLoader::next_unchecked(account_info_iter)?;

        Ok(InitMiningAccountContext {
            internal_mining,
            liquidity_mint,
            collateral_mint,
            depositor,
            depositor_authority,
            staking_program_id,
            registry,
            manager,
            rent,
            system_program,
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

        let seeds = {
            let pda = InternalMiningPDA {
                liquidity_mint: *self.liquidity_mint.key,
                collateral_mint: *self.collateral_mint.key,
                depositor: *self.depositor.key,
            };
            let (internal_mining_pubkey, bump) = pda.find_address(program_id);
            assert_account_key(self.internal_mining, &internal_mining_pubkey)?;
            pda.get_signing_seeds(bump)
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
            cpi::system::create_account::<InternalMining>(
                program_id,
                self.manager.clone(),
                self.internal_mining.clone(),
                &[&seeds.as_seeds_slice()],
                rent,
            )?;
        } else {
            assert_owned_by(self.internal_mining, program_id)?;
            // Check that account
            InternalMining::unpack(&self.internal_mining.data.borrow())?;
        }

        match mining_type {
            MiningType::Larix {
                mining_account,
                additional_reward_token_account,
            } => {
                {
                    let registry_markets =
                        everlend_registry::state::RegistryMarkets::unpack_from_slice(
                            &self.registry.data.borrow(),
                        )?;
                    if !registry_markets
                        .money_markets
                        .contains(self.staking_program_id.key)
                    {
                        return Err(ProgramError::InvalidArgument);
                    }
                }

                let mining_account_info =
                    AccountLoader::next_with_key(account_info_iter, &mining_account)?;
                assert_owned_by(mining_account_info, self.staking_program_id.key)?;

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
                    self.staking_program_id.key,
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
                assert_account_key(self.staking_program_id, &staking_program_id)?;
                let staking_pool_info =
                    AccountLoader::next_with_key(account_info_iter, &staking_pool)?;
                let staking_account_info =
                    AccountLoader::next_with_key(account_info_iter, &staking_account)?;

                assert_owned_by(staking_account_info, self.staking_program_id.key)?;

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
                    self.staking_program_id.key,
                    staking_account_info.clone(),
                    staking_pool_info.clone(),
                    self.depositor_authority.clone(),
                    self.rent.clone(),
                )?;
            }
            MiningType::Quarry { rewarder } => {
                assert_account_key(self.staking_program_id, &cpi::quarry::staking_program_id())?;

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
                    self.staking_program_id.key,
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
            MiningType::Francium {
                farming_pool,
                user_reward_a,
                user_reward_b,
                user_stake_token_account,
            } => {
                let farming_pool_info =
                    AccountLoader::next_with_key(account_info_iter, &farming_pool)?;
                let user_farming_info =
                    AccountLoader::next_with_owner(account_info_iter, &self.system_program.key)?;

                let user_farming = francium::find_user_farming_address(
                    self.depositor_authority.key,
                    &farming_pool,
                    &user_stake_token_account,
                );

                assert_account_key(user_farming_info, &user_farming)?;
                let user_reward_a_info =
                    AccountLoader::next_with_key(account_info_iter, &user_reward_a)?;
                let user_reward_b_info =
                    AccountLoader::next_with_key(account_info_iter, &user_reward_b)?;

                {
                    let farming_pool =
                        francium::FarmingPool::try_from_slice(&farming_pool_info.data.borrow())?;

                    let (user_reward_a_check, _) = find_transit_program_address(
                        program_id,
                        &self.depositor.key,
                        &farming_pool.rewards_token_mint,
                        francium::FRANCIUM_REWARD_SEED,
                    );

                    assert_account_key(&user_reward_a_info, &user_reward_a_check)?;

                    let (user_reward_b_check, _) = find_transit_program_address(
                        program_id,
                        &self.depositor.key,
                        &farming_pool.rewards_token_mint_b,
                        francium::FRANCIUM_REWARD_SEED,
                    );

                    assert_account_key(&user_reward_b_info, &user_reward_b_check)?;
                }

                let user_stake_info =
                    AccountLoader::next_with_key(account_info_iter, &user_stake_token_account)?;
                let (user_stake, _) = find_transit_program_address(
                    program_id,
                    &self.depositor.key,
                    &self.collateral_mint.key,
                    "",
                );

                assert_account_key(user_stake_info, &user_stake)?;

                cpi::francium::init_farming_user(
                    self.staking_program_id.key,
                    self.depositor_authority.clone(),
                    user_farming_info.clone(),
                    farming_pool_info.clone(),
                    user_stake_info.clone(),
                    user_reward_a_info.clone(),
                    user_reward_b_info.clone(),
                    self.system_program.clone(),
                    self.rent.clone(),
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
