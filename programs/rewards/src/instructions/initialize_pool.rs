use solana_program::account_info::AccountInfo;
use solana_program::entrypoint_deprecated::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::{system_program};
use solana_program::sysvar::{Sysvar, SysvarId};
use everlend_utils::{AccountLoader, assert_account_key};
use crate::find_reward_pool_program_address;
use crate::state::{InitRewardPoolParams, RewardPool, RootAccount};

/// Instruction context
pub struct InitializePoolContext<'a, 'b> {
    root_account: &'a AccountInfo<'b>,
    reward_pool: &'a AccountInfo<'b>,
    liquidity_mint: &'a AccountInfo<'b>,
    deposit_authority: &'a AccountInfo<'b>,
    payer: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitializePoolContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<InitializePoolContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let root_account = AccountLoader::next_unchecked(account_info_iter)?;
        let reward_pool = AccountLoader::next_uninitialized(account_info_iter)?;
        let liquidity_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let deposit_authority = AccountLoader::next_unchecked(account_info_iter)?;
        let payer = AccountLoader::next_signer(account_info_iter)?;
        let _token_program =
            AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;

        Ok(
            InitializePoolContext {
                root_account,
                reward_pool,
                liquidity_mint,
                deposit_authority,
                payer,
                rent
            }
        )
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let rent = Rent::from_account_info(self.rent)?;

        let (reward_pool_pubkey, bump) = find_reward_pool_program_address(
            program_id,
            self.root_account.key,
            self.liquidity_mint.key
        );

        {
            let root_account = RootAccount::unpack(&self.root_account.data.borrow())?;

            assert_account_key(self.payer, &root_account.authority)?;
            assert_account_key(self.reward_pool, &reward_pool_pubkey)?;
        }

        let reward_pool_seeds = &[
            "reward_pool".as_bytes(),
            self.root_account.key.as_ref(),
            self.liquidity_mint.key.as_ref(),
            &[bump]
        ];

        everlend_utils::cpi::system::create_account::<RewardPool>(
            program_id,
            self.payer.clone(),
            self.reward_pool.clone(),
            &[reward_pool_seeds],
            &rent
        )?;

        let reward_pool = RewardPool::init(
            InitRewardPoolParams {
                root_account: *self.root_account.key,
                bump,
                liquidity_mint: *self.liquidity_mint.key,
                deposit_authority: *self.deposit_authority.key
            }
        );
        RewardPool::pack(
            reward_pool,
            *self.reward_pool.data.borrow_mut(),
        )?;

        Ok(())
    }
}
