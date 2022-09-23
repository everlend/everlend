use crate::find_reward_pool_program_address;
use crate::state::{DeprecatedRewardPool, RewardPool, RewardsRoot};
use everlend_utils::{assert_account_key, AccountLoader, EverlendError};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint_deprecated::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_program;
use solana_program::sysvar::{Sysvar, SysvarId};

/// Instruction context
pub struct MigratePoolContext<'a, 'b> {
    rewards_root: &'a AccountInfo<'b>,
    reward_pool: &'a AccountInfo<'b>,
    liquidity_mint: &'a AccountInfo<'b>,
    payer: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> MigratePoolContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<MigratePoolContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let rewards_root = AccountLoader::next_with_owner(account_info_iter, &program_id)?;
        let reward_pool = AccountLoader::next_with_owner(account_info_iter, &program_id)?;
        let liquidity_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let payer = AccountLoader::next_signer(account_info_iter)?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;

        Ok(MigratePoolContext {
            rewards_root,
            reward_pool,
            liquidity_mint,
            payer,
            rent,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let rent = Rent::from_account_info(self.rent)?;

        let deprecated_pool = DeprecatedRewardPool::unpack(&self.reward_pool.data.borrow())?;
        let reward_pool = RewardPool::migrate(&deprecated_pool);

        let (reward_pool_pubkey, bump) = find_reward_pool_program_address(
            program_id,
            self.rewards_root.key,
            self.liquidity_mint.key,
        );
        {
            let rewards_root = RewardsRoot::unpack(&self.rewards_root.data.borrow())?;
            assert_account_key(self.payer, &rewards_root.authority)?;
            assert_account_key(self.reward_pool, &reward_pool_pubkey)?;
            assert_account_key(self.rewards_root, &deprecated_pool.rewards_root)?;
            assert_account_key(self.liquidity_mint, &deprecated_pool.liquidity_mint)?;
        }

        // Close pool account and return rent
        let deprecated_pool_lamports = self.reward_pool.lamports();

        **self.reward_pool.lamports.borrow_mut() = 0;
        **self.payer.lamports.borrow_mut() = self
            .payer
            .lamports()
            .checked_add(deprecated_pool_lamports)
            .ok_or(EverlendError::MathOverflow)?;

        let reward_pool_seeds = &[
            "reward_pool".as_bytes(),
            self.rewards_root.key.as_ref(),
            self.liquidity_mint.key.as_ref(),
            &[bump],
        ];

        everlend_utils::cpi::system::create_account::<RewardPool>(
            program_id,
            self.payer.clone(),
            self.reward_pool.clone(),
            &[reward_pool_seeds],
            &rent,
        )?;

        RewardPool::pack(reward_pool, *self.reward_pool.data.borrow_mut())?;

        Ok(())
    }
}
