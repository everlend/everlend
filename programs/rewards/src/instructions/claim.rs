use crate::state::{Mining, RewardPool};
use everlend_utils::{assert_account_key, AccountLoader};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

/// Instruction context
pub struct ClaimContext<'a, 'b> {
    reward_pool: &'a AccountInfo<'b>,
    reward_mints: Vec<&'a AccountInfo<'b>>,
    vaults: Vec<&'a AccountInfo<'b>>,
    mining: &'a AccountInfo<'b>,
    user: &'a AccountInfo<'b>,
    user_reward_token_accounts: Vec<&'a AccountInfo<'b>>,
}

impl<'a, 'b> ClaimContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<ClaimContext<'a, 'b>, ProgramError> {
        let mut reward_mints = vec![];
        let mut vaults = vec![];
        let mut user_reward_token_accounts = vec![];

        let account_info_iter = &mut accounts.iter().enumerate();
        let reward_pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let reward_pool_unpack = RewardPool::unpack(&reward_pool.data.borrow())?;

        for _ in reward_pool_unpack.vaults.iter() {
            let reward_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
            reward_mints.push(reward_mint)
        }
        for _ in reward_pool_unpack.vaults.iter() {
            let vault = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
            vaults.push(vault)
        }
        let mining = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let user = AccountLoader::next_signer(account_info_iter)?;

        for _ in reward_pool_unpack.vaults.iter() {
            let user_reward_token_account =
                AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
            user_reward_token_accounts.push(user_reward_token_account)
        }

        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;

        Ok(ClaimContext {
            reward_pool,
            reward_mints,
            vaults,
            mining,
            user,
            user_reward_token_accounts,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let reward_pool = RewardPool::unpack(&self.reward_pool.data.borrow())?;
        let mut mining = Mining::unpack(&self.mining.data.borrow())?;

        let reward_pool_seeds = &[
            b"reward_pool".as_ref(),
            &reward_pool.rewards_root.to_bytes()[..32],
            &reward_pool.liquidity_mint.to_bytes()[..32],
            &[reward_pool.bump],
        ];

        {
            assert_account_key(self.user, &mining.owner)?;
            assert_account_key(self.reward_pool, &mining.reward_pool)?;
            assert_account_key(
                self.reward_pool,
                &Pubkey::create_program_address(reward_pool_seeds, program_id)?,
            )?;
            for i in 0..=(reward_pool.vaults.len() - 1) {
                let bump = reward_pool
                    .vaults
                    .iter()
                    .find(|v| &v.reward_mint == self.reward_mints[i].key)
                    .ok_or(ProgramError::InvalidArgument)?
                    .bump;
                let vault_seeds = &[
                    b"vault".as_ref(),
                    &self.reward_pool.key.to_bytes()[..32],
                    &self.reward_mints[i].key.to_bytes()[..32],
                    &[bump],
                ];
                assert_account_key(
                    self.vaults[i],
                    &Pubkey::create_program_address(vault_seeds, program_id)?,
                )?;
            }
        }

        mining.refresh_rewards(reward_pool.vaults.iter())?;
        for i in 0..=(reward_pool.vaults.len() - 1) {
            let mut reward_index = mining.reward_index_mut(*self.reward_mints[i].key);
            let amount = reward_index.rewards;

            reward_index.rewards = 0;

            everlend_utils::cpi::spl_token::transfer(
                self.vaults[i].clone(),
                self.user_reward_token_accounts[i].clone(),
                self.reward_pool.clone(),
                amount,
                &[reward_pool_seeds],
            )?;
        }
        Mining::pack(mining, *self.mining.data.borrow_mut())?;

        Ok(())
    }
}
