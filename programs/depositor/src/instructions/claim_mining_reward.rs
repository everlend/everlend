use crate::claimer::{
    FranciumClaimer, LarixClaimer, PortFinanceClaimer, QuarryClaimer, RewardClaimer,
};
use crate::{
    find_internal_mining_program_address,
    state::{Depositor, InternalMining, MiningType},
    utils::{parse_fill_reward_accounts, FillRewardAccounts},
};
use borsh::BorshDeserialize;
use everlend_rewards::{cpi::fill_vault, state::RewardPool};
use everlend_utils::cpi::francium;
use everlend_utils::{assert_account_key, find_program_address, AccountLoader, EverlendError};
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

        let claimer: Box<dyn RewardClaimer<'b> + 'a> = {
            match internal_mining_type {
                MiningType::Larix { .. } => {
                    reward_accounts
                        .check_transit_reward_destination(program_id, self.depositor.key)?;

                    //Larix has manual distribution of subreward so we dont need this check
                    // fill_sub_rewards_accounts.check_transit_reward_destination()?;

                    let larix = LarixClaimer::init(
                        self.staking_program_id.key,
                        internal_mining_type,
                        with_subrewards,
                        fill_sub_rewards_accounts.clone(),
                        account_info_iter,
                    )?;

                    Box::new(larix)
                }
                MiningType::PortFinance { .. } => {
                    reward_accounts
                        .check_transit_reward_destination(program_id, self.depositor.key)?;

                    if with_subrewards {
                        fill_sub_rewards_accounts
                            .as_ref()
                            .unwrap()
                            .check_transit_reward_destination(program_id, self.depositor.key)?;
                    };

                    let port_finance = PortFinanceClaimer::init(
                        self.staking_program_id.key,
                        internal_mining_type,
                        with_subrewards,
                        fill_sub_rewards_accounts.clone(),
                        account_info_iter,
                    )?;

                    Box::new(port_finance)
                }
                MiningType::Quarry { .. } => {
                    reward_accounts
                        .check_transit_reward_destination(program_id, self.depositor.key)?;

                    // Quarry doesn't have subreward tokens
                    if with_subrewards {
                        return Err(ProgramError::InvalidArgument);
                    }

                    let quarry = QuarryClaimer::init(
                        program_id,
                        self.depositor.key,
                        self.depositor_authority.key,
                        self.collateral_mint.key,
                        self.staking_program_id.key,
                        internal_mining_type,
                        account_info_iter,
                    )?;

                    Box::new(quarry)
                }
                MiningType::Francium {
                    user_reward_a,
                    user_reward_b,
                    farming_pool,
                    ..
                } => {
                    let farming_pool =
                        AccountLoader::next_with_key(account_info_iter, &farming_pool)?;
                    let farming_pool_unpack: francium::FarmingPool =
                        francium::FarmingPool::try_from_slice(&farming_pool.data.borrow())?;

                    if farming_pool_unpack.is_dual_rewards != with_subrewards {
                        return Err(ProgramError::InvalidArgument);
                    }
                    let check: bool = farming_pool_unpack.rewards_per_day != 0
                        && farming_pool_unpack.rewards_start_slot
                            != farming_pool_unpack.rewards_end_slot;

                    if check {
                        assert_account_key(reward_accounts.reward_transit_info, &user_reward_a)?;
                    } else {
                        assert_account_key(reward_accounts.reward_transit_info, &user_reward_b)?;
                    }

                    if with_subrewards {
                        if check {
                            assert_account_key(
                                fill_sub_rewards_accounts
                                    .as_ref()
                                    .unwrap()
                                    .reward_transit_info,
                                &user_reward_b,
                            )?;
                        } else {
                            assert_account_key(
                                fill_sub_rewards_accounts
                                    .as_ref()
                                    .unwrap()
                                    .reward_transit_info,
                                &user_reward_a,
                            )?;
                        }
                    };

                    let francium = FranciumClaimer::init(
                        program_id,
                        self.staking_program_id.key,
                        self.depositor_authority.key,
                        self.depositor.key,
                        internal_mining_type,
                        fill_sub_rewards_accounts.clone(),
                        farming_pool,
                        account_info_iter,
                    )?;

                    Box::new(francium)
                }
                _ => return Err(EverlendError::MiningNotInitialized.into()),
            }
        };

        claimer.claim_reward(
            self.staking_program_id.key,
            reward_accounts.reward_transit_info.clone(),
            self.depositor_authority.clone(),
            &[signers_seeds.as_ref()],
        )?;

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
