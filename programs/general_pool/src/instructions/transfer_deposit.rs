use everlend_rewards::state::Mining;
use everlend_utils::{
    assert_account_key,
    cpi::{self},
    AccountLoader, EverlendError,
};
use everlend_rewards::cpi::{deposit_mining, withdraw_mining};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError, program_pack::Pack, pubkey::Pubkey};
use spl_token::state::Account;

use crate::{find_pool_program_address, find_user_mining_address, state::Pool};

/// Instruction context
pub struct TransferDepositContext<'a, 'b> {
    pool: &'a AccountInfo<'b>,
    source: &'a AccountInfo<'b>,
    destination: &'a AccountInfo<'b>,
    user_authority: &'a AccountInfo<'b>,
    destination_user_authority: &'a AccountInfo<'b>,
    mining_reward_pool: &'a AccountInfo<'b>,
    mining_reward_acc: &'a AccountInfo<'b>,
    destination_mining_reward_acc: &'a AccountInfo<'b>,
    everlend_config: &'a AccountInfo<'b>,
    everlend_rewards: &'a AccountInfo<'b>,
}

impl<'a, 'b> TransferDepositContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<TransferDepositContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let source = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let destination = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let user_authority = AccountLoader::next_signer(account_info_iter)?;
        let destination_user_authority = AccountLoader::next_unchecked(account_info_iter)?;

        // mining accounts
        let mining_reward_pool =
            AccountLoader::next_with_owner(account_info_iter, &everlend_rewards::id())?;
        let mining_reward_acc =
            AccountLoader::next_with_owner(account_info_iter, &everlend_rewards::id())?;
        let destination_mining_reward_acc =
            AccountLoader::next_with_owner(account_info_iter, &everlend_rewards::id())?;
        let everlend_config = AccountLoader::next_with_owner(account_info_iter, &everlend_rewards::id())?;
        let everlend_rewards = AccountLoader::next_with_key(account_info_iter, &everlend_rewards::id())?;
        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;

        Ok(TransferDepositContext {
            pool,
            source,
            destination,
            user_authority,
            destination_user_authority,
            mining_reward_pool,
            mining_reward_acc,
            destination_mining_reward_acc,
            everlend_rewards,
            everlend_config,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        // Get pool state
        let pool = Pool::unpack(&self.pool.data.borrow())?;
        let source_account = Account::unpack(&self.source.data.borrow())?;

        // Check pool accounts
        {
            let destination_account = Account::unpack(&self.destination.data.borrow())?;

            if source_account.mint != pool.pool_mint || destination_account.mint != pool.pool_mint {
                return Err(ProgramError::InvalidArgument);
            }

            let (mining_reward_acc_pubkey, _) =
                find_user_mining_address(self.user_authority.key, self.mining_reward_pool.key);
            assert_account_key(self.mining_reward_acc, &mining_reward_acc_pubkey)?;
        }

        let collateral_amount = source_account.amount;
        let reward_share =
            Mining::unpack(&mut self.mining_reward_acc.data.borrow().as_ref())?.share;

        if collateral_amount != reward_share {
            return Err(EverlendError::RewardAndCollateralMismatch.into());
        }

        // Transfer token from source to destination token account
        cpi::spl_token::transfer(
            self.source.clone(),
            self.destination.clone(),
            self.user_authority.clone(),
            collateral_amount,
            &[],
        )?;

        self.transfer_mining(program_id, &pool, reward_share)?;

        Ok(())
    }

    fn transfer_mining(
        &self,
        program_id: &Pubkey,
        pool: &Pool,
        transfer_amount: u64,
    ) -> ProgramResult {
        let (pool_pubkey, pool_bump_seed) =
            find_pool_program_address(program_id, &pool.pool_market, &pool.token_mint);
        assert_account_key(self.pool, &pool_pubkey)?;

        let pool_seeds: &[&[u8]] = &[
            &pool.pool_market.to_bytes()[..32],
            &pool.token_mint.to_bytes()[..32],
            &[pool_bump_seed],
        ];

        withdraw_mining(
            self.everlend_rewards.key,
            self.everlend_config.clone(),
            self.mining_reward_pool.clone(),
            self.mining_reward_acc.clone(),
            self.user_authority.clone(),
            self.pool.to_owned(),
            transfer_amount,
            &[pool_seeds],
        )?;

        deposit_mining(
            self.everlend_rewards.key,
            self.everlend_config.clone(),
            self.mining_reward_pool.clone(),
            self.destination_mining_reward_acc.clone(),
            self.destination_user_authority.clone(),
            self.pool.to_owned(),
            transfer_amount,
            &[pool_seeds],
        )
    }
}
