use everlend_utils::{
    assert_account_key,
    cpi::{self},
    find_program_address, AccountLoader, EverlendError,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Mint;
use everlend_rewards::cpi::deposit_mining;

use crate::{
    find_pool_config_program_address, find_pool_program_address,
    state::{Pool, PoolConfig},
    utils::total_pool_amount,
};

/// Instruction context
pub struct DepositContext<'a, 'b> {
    destination: &'a AccountInfo<'b>,
    everlend_rewards: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    pool_config: &'a AccountInfo<'b>,
    pool_market: &'a AccountInfo<'b>,
    pool_market_authority: &'a AccountInfo<'b>,
    pool_mint: &'a AccountInfo<'b>,
    source: &'a AccountInfo<'b>,
    token_account: &'a AccountInfo<'b>,
    user_transfer_authority: &'a AccountInfo<'b>,
    mining_reward_pool: &'a AccountInfo<'b>,
    mining_reward_acc: &'a AccountInfo<'b>,
    everlend_config: &'a AccountInfo<'b>,
}

impl<'a, 'b> DepositContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<DepositContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let pool_config = AccountLoader::next_optional(account_info_iter, program_id)?;
        let pool_market = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;

        let source = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let destination = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let token_account = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let pool_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let pool_market_authority = AccountLoader::next_unchecked(account_info_iter)?; // Is PDA account of this program
        let user_transfer_authority = AccountLoader::next_signer(account_info_iter)?;

        // mining accounts
        let mining_reward_pool =
            AccountLoader::next_with_owner(account_info_iter, &everlend_rewards::id())?;
        let mining_reward_acc =
            AccountLoader::next_with_owner(account_info_iter, &everlend_rewards::id())?;
        let everlend_config = AccountLoader::next_with_owner(account_info_iter, &everlend_rewards::id())?;
        let everlend_rewards = AccountLoader::next_with_key(account_info_iter, &everlend_rewards::id())?;
        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;

        Ok(DepositContext {
            destination,
            everlend_rewards,
            pool,
            pool_config,
            pool_market,
            pool_market_authority,
            pool_mint,
            source,
            token_account,
            user_transfer_authority,
            mining_reward_pool,
            mining_reward_acc,
            everlend_config,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey, amount: u64) -> ProgramResult {
        // Get pool state
        let pool = Pool::unpack(&self.pool.data.borrow())?;

        // Check pool accounts
        assert_account_key(self.pool_market, &pool.pool_market)?;
        assert_account_key(self.token_account, &pool.token_account)?;
        assert_account_key(self.pool_mint, &pool.pool_mint)?;

        {
            let (pool_config_pubkey, _) =
                find_pool_config_program_address(program_id, self.pool.key);
            assert_account_key(self.pool_config, &pool_config_pubkey)?;

            // Check only if account exists
            if !self.pool_config.owner.eq(&Pubkey::default()) {
                let pool_config = PoolConfig::unpack(&self.pool_config.data.borrow())?;
                if amount < pool_config.deposit_minimum {
                    return Err(EverlendError::DepositAmountTooSmall.into());
                }
            }
        }

        let total_incoming =
            total_pool_amount(self.token_account.clone(), pool.total_amount_borrowed)?;
        let total_minted = Mint::unpack_unchecked(&self.pool_mint.data.borrow())?.supply;

        let mint_amount = if total_incoming == 0 || total_minted == 0 {
            amount
        } else {
            (amount as u128)
                .checked_mul(total_minted as u128)
                .ok_or(ProgramError::InvalidArgument)?
                .checked_div(total_incoming as u128)
                .ok_or(ProgramError::InvalidArgument)? as u64
        };

        if mint_amount == 0 {
            return Err(EverlendError::DepositAmountTooSmall.into());
        }

        self.transfer_and_mint(program_id, amount, mint_amount)?;
        self.deposit_mining(program_id, &pool, mint_amount)?;

        Ok(())
    }

    fn transfer_and_mint(
        &self,
        program_id: &Pubkey,
        amount: u64,
        mint_amount: u64,
    ) -> ProgramResult {
        // Transfer token from source to token account
        cpi::spl_token::transfer(
            self.source.clone(),
            self.token_account.clone(),
            self.user_transfer_authority.clone(),
            amount,
            &[],
        )?;

        let (_, bump_seed) = find_program_address(program_id, self.pool_market.key);
        let signers_seeds = &[&self.pool_market.key.to_bytes()[..32], &[bump_seed]];

        // Mint to destination pool token
        cpi::spl_token::mint_to(
            self.pool_mint.clone(),
            self.destination.clone(),
            self.pool_market_authority.clone(),
            mint_amount,
            &[signers_seeds],
        )
    }

    fn deposit_mining(&self, program_id: &Pubkey, pool: &Pool, mint_amount: u64) -> ProgramResult {
        let (pool_pubkey, pool_bump_seed) =
            find_pool_program_address(program_id, &pool.pool_market, &pool.token_mint);
        assert_account_key(self.pool, &pool_pubkey)?;

        let pool_seeds: &[&[u8]] = &[
            &pool.pool_market.to_bytes()[..32],
            &pool.token_mint.to_bytes()[..32],
            &[pool_bump_seed],
        ];

        deposit_mining(
            self.everlend_rewards.key,
            self.everlend_config.clone(),
            self.mining_reward_pool.clone(),
            self.mining_reward_acc.clone(),
            self.user_transfer_authority.clone(),
            self.pool.to_owned(),
            mint_amount,
            &[pool_seeds],
        )
    }
}
