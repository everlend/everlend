use crate::{
    find_internal_mining_program_address,
    state::{Depositor, InternalMining, MiningType},
    utils::{parse_fill_reward_accounts, FillRewardAccounts},
};
use everlend_utils::{
    assert_account_key,
    cpi::{larix, port_finance, quarry, rewards},
    find_program_address, AccountLoader,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey, sysvar::clock,
};
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct ClaimMiningRewardsContext<'a, 'b> {
    depositor: &'a AccountInfo<'b>,
    depositor_authority: &'a AccountInfo<'b>,
    executor: &'a AccountInfo<'b>,
    liquidity_mint: &'a AccountInfo<'b>,
    collateral_mint: &'a AccountInfo<'b>,
    internal_mining: &'a AccountInfo<'b>,
    staking_program_id: &'a AccountInfo<'b>,
    eld_reward_program_id: &'a AccountInfo<'b>,
    eld_config: &'a AccountInfo<'b>,
    reward_pool: &'a AccountInfo<'b>,
}

impl<'a, 'b> ClaimMiningRewardsContext<'a, 'b> {
    /// New ClaimMiningReward instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<ClaimMiningRewardsContext<'a, 'b>, ProgramError> {
        let depositor = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let depositor_authority = AccountLoader::next_unchecked(account_info_iter)?; //depositor PDA signer
        let executor = AccountLoader::next_signer(account_info_iter)?;
        let liquidity_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let collateral_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let internal_mining = AccountLoader::next_with_owner(account_info_iter, program_id)?;

        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;
        let staking_program_id = AccountLoader::next_unchecked(account_info_iter)?;

        let eld_reward_program_id =
            AccountLoader::next_with_key(account_info_iter, &eld_rewards::id())?;
        let eld_config = AccountLoader::next_with_owner(account_info_iter, &eld_config::id())?;

        let reward_pool = AccountLoader::next_with_owner(account_info_iter, &eld_rewards::id())?;

        Ok(ClaimMiningRewardsContext {
            depositor,
            depositor_authority,
            executor,
            liquidity_mint,
            collateral_mint,
            internal_mining,
            staking_program_id,
            eld_reward_program_id,
            eld_config,
            reward_pool,
        })
    }

    /// Process ClaimMiningReward instruction
    pub fn process(
        &self,
        program_id: &Pubkey,
        account_info_iter: &'a mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        with_subrewards: bool,
    ) -> ProgramResult {
        {
            let depositor = Depositor::unpack(&self.depositor.data.borrow())?;
            assert_account_key(self.executor, &depositor.rebalance_executor)?;
        }

        {
            let (internal_mining_pubkey, _) = find_internal_mining_program_address(
                program_id,
                self.liquidity_mint.key,
                self.collateral_mint.key,
                self.depositor.key,
            );
            assert_account_key(self.internal_mining, &internal_mining_pubkey)?;
        }

        let internal_mining_type =
            InternalMining::unpack(&self.internal_mining.data.borrow())?.mining_type;

        // TODO fix unpack and check liquidity mint
        // let reward_pool = RewardPool::try_from_slice(&self.reward_pool.data.borrow()[8..])?;
        // assert_account_key(self.liquidity_mint, &reward_pool.liquidity_mint)?;

        let reward_accounts = parse_fill_reward_accounts(
            program_id,
            self.depositor.key,
            self.reward_pool.key,
            self.eld_reward_program_id.key,
            account_info_iter,
            true,
        )?;

        let mut fill_sub_rewards_accounts: Option<FillRewardAccounts> = None;

        let signers_seeds = {
            // Create depositor authority account
            let (depositor_authority_pubkey, bump_seed) =
                find_program_address(program_id, self.depositor.key);
            assert_account_key(self.depositor_authority, &depositor_authority_pubkey)?;
            &[&self.depositor.key.to_bytes()[..32], &[bump_seed]]
        };

        match internal_mining_type {
            MiningType::Larix {
                mining_account,
                additional_reward_token_account,
            } => {
                if with_subrewards != additional_reward_token_account.is_some() {
                    return Err(ProgramError::InvalidArgument);
                };

                // Parse and check additional reward token account
                if with_subrewards {
                    let sub_reward_accounts = parse_fill_reward_accounts(
                        program_id,
                        self.depositor.key,
                        self.reward_pool.key,
                        self.eld_reward_program_id.key,
                        account_info_iter,
                        //Larix has manual distribution of subreward
                        false,
                    )?;

                    // Assert additional reward token account
                    assert_account_key(
                        &sub_reward_accounts.reward_transit_info,
                        &additional_reward_token_account.unwrap(),
                    )?;

                    fill_sub_rewards_accounts = Some(sub_reward_accounts);
                };

                let mining_account_info =
                    AccountLoader::next_with_key(account_info_iter, &mining_account)?;

                let mine_supply_info = AccountLoader::next_unchecked(account_info_iter)?;
                let lending_market_info = AccountLoader::next_unchecked(account_info_iter)?;
                let lending_market_authority_info =
                    AccountLoader::next_unchecked(account_info_iter)?;
                let reserve_info = AccountLoader::next_unchecked(account_info_iter)?;
                let reserve_liquidity_oracle = AccountLoader::next_unchecked(account_info_iter)?;

                larix::refresh_mine(
                    self.staking_program_id.key,
                    mining_account_info.clone(),
                    reserve_info.clone(),
                )?;

                larix::refresh_reserve(
                    self.staking_program_id.key,
                    reserve_info.clone(),
                    reserve_liquidity_oracle.clone(),
                )?;

                larix::claim_mine(
                    self.staking_program_id.key,
                    mining_account_info.clone(),
                    mine_supply_info.clone(),
                    reward_accounts.reward_transit_info.clone(),
                    self.depositor_authority.clone(),
                    lending_market_info.clone(),
                    lending_market_authority_info.clone(),
                    reserve_info.clone(),
                    &[signers_seeds.as_ref()],
                )?;
            }
            MiningType::PortFinance {
                staking_account,
                staking_pool,
                staking_program_id,
                ..
            } => {
                if with_subrewards {
                    let sub_reward_accounts = parse_fill_reward_accounts(
                        program_id,
                        self.depositor.key,
                        self.reward_pool.key,
                        self.eld_reward_program_id.key,
                        account_info_iter,
                        true,
                    )?;
                    fill_sub_rewards_accounts = Some(sub_reward_accounts.clone());
                }
                assert_account_key(self.staking_program_id, &staking_program_id)?;

                let stake_account_info =
                    AccountLoader::next_with_key(account_info_iter, &staking_account)?;
                let staking_pool_info =
                    AccountLoader::next_with_key(account_info_iter, &staking_pool)?;
                let staking_pool_authority_info = AccountLoader::next_unchecked(account_info_iter)?;

                let reward_token_pool = AccountLoader::next_unchecked(account_info_iter)?;

                let clock = AccountLoader::next_with_key(account_info_iter, &clock::id())?;

                // let sub_reward_token_pool_option :Option<AccountInfo>;
                // let sub_reward_destination_option :Option<AccountInfo>;
                let sub_reward = if with_subrewards {
                    let sub_reward_token_pool = AccountLoader::next_unchecked(account_info_iter)?;

                    // Make local copy
                    let sub_reward_destination = fill_sub_rewards_accounts.unwrap().clone();
                    fill_sub_rewards_accounts = Some(sub_reward_destination.clone());

                    Some((
                        sub_reward_token_pool,
                        sub_reward_destination.reward_transit_info,
                    ))
                } else {
                    None
                };

                port_finance::claim_reward(
                    self.staking_program_id.key,
                    self.depositor_authority.clone(),
                    stake_account_info.clone(),
                    staking_pool_info.clone(),
                    staking_pool_authority_info.clone(),
                    reward_token_pool.clone(),
                    reward_accounts.reward_transit_info.clone(),
                    sub_reward,
                    clock.clone(),
                    &[signers_seeds.as_ref()],
                )?;
            }
            MiningType::Quarry {
                quarry_mining_program_id,
                quarry,
                rewarder,
                miner_vault: _,
            } => {
                assert_account_key(self.staking_program_id, &quarry_mining_program_id)?;
                // TODO add checks
                let mint_wrapper = AccountLoader::next_unchecked(account_info_iter)?;
                let mint_wrapper_program = AccountLoader::next_unchecked(account_info_iter)?;
                let minter = AccountLoader::next_unchecked(account_info_iter)?;
                let rewards_token_mint = AccountLoader::next_unchecked(account_info_iter)?;
                let rewards_token_account = AccountLoader::next_unchecked(account_info_iter)?;
                let rewards_fee_account = AccountLoader::next_unchecked(account_info_iter)?;
                let miner = AccountLoader::next_unchecked(account_info_iter)?;

                let quarry_info = AccountLoader::next_with_key(account_info_iter, &quarry)?;
                let quarry_rewarder = AccountLoader::next_with_key(account_info_iter, &rewarder)?;

                quarry::claim_rewards(
                    self.staking_program_id.key,
                    mint_wrapper.clone(),
                    mint_wrapper_program.clone(),
                    minter.clone(),
                    rewards_token_mint.clone(),
                    rewards_token_account.clone(),
                    rewards_fee_account.clone(),
                    self.depositor_authority.clone(),
                    miner.clone(),
                    quarry_info.clone(),
                    quarry_rewarder.clone(),
                )?;
            }
            MiningType::None => {}
        };

        let mut fill_itr = vec![reward_accounts];

        if let Some(accounts) = fill_sub_rewards_accounts {
            fill_itr.push(accounts);
        }

        fill_itr.iter().try_for_each(|reward_accounts| {
            let reward_transit_account =
                Account::unpack(&reward_accounts.reward_transit_info.data.borrow())?;

            rewards::fill_vault(
                self.eld_reward_program_id.key,
                self.eld_config.clone(),
                self.reward_pool.clone(),
                reward_accounts.reward_mint_info.clone(),
                reward_accounts.fee_account_info.clone(),
                reward_accounts.vault_info.clone(),
                reward_accounts.reward_transit_info.clone(),
                self.depositor_authority.clone(),
                reward_transit_account.amount,
                &[signers_seeds.as_ref()],
            )
        })?;

        Ok(())
    }
}
