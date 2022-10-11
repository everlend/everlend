use crate::claimer::{LarixClaimer, PortFinanceClaimer, RewardClaimer};
use crate::{
    find_internal_mining_program_address, find_transit_program_address,
    state::{Depositor, InternalMining, MiningType},
    utils::{parse_fill_reward_accounts, FillRewardAccounts},
};
use everlend_rewards::{cpi::fill_vault, state::RewardPool};
use everlend_utils::{assert_account_key, cpi::quarry, find_program_address, AccountLoader};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct ClaimMiningRewardContext<'a, 'b> {
    depositor: &'a AccountInfo<'b>,
    depositor_authority: &'a AccountInfo<'b>,
    executor: &'a AccountInfo<'b>,
    liquidity_mint: &'a AccountInfo<'b>,
    collateral_mint: &'a AccountInfo<'b>,
    internal_mining: &'a AccountInfo<'b>,
    staking_program_id: &'a AccountInfo<'b>,
    rewards_program_id: &'a AccountInfo<'b>,
    reward_pool: &'a AccountInfo<'b>,
}

impl<'a, 'b> ClaimMiningRewardContext<'a, 'b> {
    /// New ClaimMiningReward instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<ClaimMiningRewardContext<'a, 'b>, ProgramError> {
        let depositor = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let depositor_authority = AccountLoader::next_unchecked(account_info_iter)?; //depositor PDA signer
        let executor = AccountLoader::next_signer(account_info_iter)?;
        let liquidity_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let collateral_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let internal_mining = AccountLoader::next_with_owner(account_info_iter, program_id)?;

        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;
        let staking_program_id = AccountLoader::next_unchecked(account_info_iter)?;

        let rewards_program_id =
            AccountLoader::next_with_key(account_info_iter, &everlend_rewards::id())?;

        let reward_pool =
            AccountLoader::next_with_owner(account_info_iter, &everlend_rewards::id())?;

        Ok(ClaimMiningRewardContext {
            depositor,
            depositor_authority,
            executor,
            liquidity_mint,
            collateral_mint,
            internal_mining,
            staking_program_id,
            rewards_program_id,
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
            assert_account_key(self.internal_mining, &internal_mining_pubkey)
        }?;

        let internal_mining_type =
            InternalMining::unpack(&self.internal_mining.data.borrow())?.mining_type;

        {
            let reward_pool = RewardPool::unpack(&self.reward_pool.data.borrow())?;
            assert_account_key(self.liquidity_mint, &reward_pool.liquidity_mint)?;
        }

        let reward_accounts = parse_fill_reward_accounts(
            self.reward_pool.key,
            self.rewards_program_id.key,
            account_info_iter,
        )?;

        reward_accounts.check_transit_reward_destination(program_id, self.depositor.key)?;

        let mut fill_sub_rewards_accounts: Option<FillRewardAccounts> = None;

        let signers_seeds = {
            // Create depositor authority account
            let (depositor_authority_pubkey, bump_seed) =
                find_program_address(program_id, self.depositor.key);
            assert_account_key(self.depositor_authority, &depositor_authority_pubkey)?;
            &[&self.depositor.key.to_bytes()[..32], &[bump_seed]]
        };

        // Parse and check additional reward token account
        if with_subrewards {
            let sub_reward_accounts = parse_fill_reward_accounts(
                self.reward_pool.key,
                self.rewards_program_id.key,
                account_info_iter,
            )?;

            fill_sub_rewards_accounts = Some(sub_reward_accounts);
        };

        match internal_mining_type {
            MiningType::Larix { .. } => {
                //Larix has manual distribution of subreward so we dont need this check
                // fill_sub_rewards_accounts.check_transit_reward_destination()?;

                let claimer = LarixClaimer::init(
                    self.staking_program_id.key,
                    internal_mining_type,
                    with_subrewards,
                    fill_sub_rewards_accounts.clone(),
                    account_info_iter,
                )?;

                claimer.claim_reward(
                    self.staking_program_id.key,
                    reward_accounts.reward_transit_info.clone(),
                    self.depositor_authority.clone(),
                    &[signers_seeds.as_ref()],
                )?;
            }
            MiningType::PortFinance { .. } => {
                if with_subrewards {
                    reward_accounts
                        .check_transit_reward_destination(program_id, self.depositor.key)?;
                };

                let claimer = PortFinanceClaimer::init(
                    self.staking_program_id.key,
                    internal_mining_type,
                    with_subrewards,
                    fill_sub_rewards_accounts.clone(),
                    account_info_iter,
                )?;

                claimer.claim_reward(
                    self.staking_program_id.key,
                    reward_accounts.reward_transit_info.clone(),
                    self.depositor_authority.clone(),
                    &[signers_seeds.as_ref()],
                )?;
            }
            MiningType::Quarry { rewarder } => {
                assert_account_key(self.staking_program_id, &quarry::staking_program_id())?;
                let mint_wrapper = AccountLoader::next_unchecked(account_info_iter)?;
                let mint_wrapper_program = AccountLoader::next_unchecked(account_info_iter)?;
                let minter = AccountLoader::next_unchecked(account_info_iter)?;
                // IOU token mint
                let rewards_token_mint = AccountLoader::next_unchecked(account_info_iter)?;

                let rewards_token_account = {
                    let (reward_token_account_pubkey, _) = find_transit_program_address(
                        program_id,
                        self.depositor.key,
                        rewards_token_mint.key,
                        "lm_reward",
                    );

                    AccountLoader::next_with_key(account_info_iter, &reward_token_account_pubkey)?
                };

                let rewards_fee_account =
                    AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

                let quarry_rewarder = AccountLoader::next_with_key(account_info_iter, &rewarder)?;

                let quarry_info = {
                    let (quarry, _) = quarry::find_quarry_program_address(
                        self.staking_program_id.key,
                        quarry_rewarder.key,
                        self.collateral_mint.key,
                    );

                    AccountLoader::next_with_key(account_info_iter, &quarry)
                }?;

                let miner = {
                    let (miner_pubkey, _) = quarry::find_miner_program_address(
                        &quarry::staking_program_id(),
                        quarry_info.key,
                        self.depositor_authority.key,
                    );

                    AccountLoader::next_with_key(account_info_iter, &miner_pubkey)
                }?;

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
                    &[signers_seeds.as_ref()],
                )?;

                let redeemer_program_id_info = AccountLoader::next_unchecked(account_info_iter)?;
                let redeemer_info = AccountLoader::next_unchecked(account_info_iter)?;
                let redemption_vault_info =
                    AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

                quarry::redeem_all_tokens(
                    redeemer_program_id_info.key,
                    redeemer_info.clone(),
                    rewards_token_mint.clone(),
                    rewards_token_account.clone(),
                    redemption_vault_info.clone(),
                    reward_accounts.reward_transit_info.clone(),
                    self.depositor_authority.clone(),
                    &[signers_seeds.as_ref()],
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

            fill_vault(
                self.rewards_program_id.key,
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
