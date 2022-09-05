use crate::{
    find_pool_program_address,
    state::{Pool, PoolMarket},
};
use everlend_utils::{
    assert_account_key,
    cpi::rewards::{deposit_mining, initialize_mining},
    next_account, next_program_account, next_signer_account, next_unchecked_account,
    next_uninitialized_account,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey, rent::Rent, system_program, sysvar::SysvarId,
};
use spl_token::state::Account;

/// Instruction context
pub struct InitUserMiningContext<'a, 'b> {
    pool_market: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    user_collateral_token_account: &'a AccountInfo<'b>,
    user_authority: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
    mining_reward_pool: &'a AccountInfo<'b>,
    mining_reward_acc: &'a AccountInfo<'b>,
    everlend_config: &'a AccountInfo<'b>,
    everlend_rewards_program: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitUserMiningContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<InitUserMiningContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let pool_market = next_account(account_info_iter, program_id)?;
        let pool = next_account(account_info_iter, program_id)?;
        let user_collateral_token_account = next_account(account_info_iter, &spl_token::id())?;
        let user_authority = next_unchecked_account(account_info_iter)?; // We don't need to check
        let manager = next_signer_account(account_info_iter)?;
        let mining_reward_pool = next_account(account_info_iter, &eld_rewards::id())?;
        let mining_reward_acc = next_uninitialized_account(account_info_iter)?;
        let everlend_config = next_account(account_info_iter, &eld_config::id())?;
        let everlend_rewards_program = next_program_account(account_info_iter, &eld_rewards::id())?;
        let system_program = next_program_account(account_info_iter, &system_program::id())?;
        let rent = next_program_account(account_info_iter, &Rent::id())?;

        Ok(InitUserMiningContext {
            pool_market,
            pool,
            user_collateral_token_account,
            user_authority,
            manager,
            mining_reward_pool,
            mining_reward_acc,
            everlend_config,
            everlend_rewards_program,
            system_program,
            rent,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let pool = Pool::unpack(&self.pool.data.borrow())?;
        assert_account_key(self.pool_market, &pool.pool_market)?;

        {
            let pool_market = PoolMarket::unpack(&self.pool_market.data.borrow())?;
            assert_account_key(self.manager, &pool_market.manager)?;
        }

        let (pool_pubkey, pool_bump_seed) =
            find_pool_program_address(program_id, &pool.pool_market, &pool.token_mint);
        assert_account_key(self.pool, &pool_pubkey)?;

        let pool_seeds: &[&[u8]] = &[
            &pool.pool_market.to_bytes()[..32],
            &pool.token_mint.to_bytes()[..32],
            &[pool_bump_seed],
        ];

        let user_account = Account::unpack(&self.user_collateral_token_account.data.borrow())?;
        if pool.pool_mint != user_account.mint {
            return Err(ProgramError::InvalidArgument);
        }

        // check authority
        if !user_account.owner.eq(self.user_authority.key) {
            return Err(ProgramError::InvalidArgument);
        }

        initialize_mining(
            self.everlend_rewards_program.key,
            self.everlend_config.clone(),
            self.mining_reward_pool.clone(),
            self.mining_reward_acc.clone(),
            self.user_authority.clone(),
            self.manager.clone(),
            self.system_program.clone(),
            self.rent.clone(),
        )?;

        deposit_mining(
            self.everlend_rewards_program.key,
            self.everlend_config.clone(),
            self.mining_reward_pool.clone(),
            self.mining_reward_acc.clone(),
            self.user_authority.clone(),
            self.pool.to_owned(),
            user_account.amount,
            &[pool_seeds],
        )?;

        Ok(())
    }
}
