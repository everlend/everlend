use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use everlend_utils::{AccountLoader, assert_account_key};
use crate::state::{Mining, RewardPool};

pub struct WithdrawMiningContext<'a, 'b> {
    config: &'a AccountInfo<'b>,
    reward_pool: &'a AccountInfo<'b>,
    mining: &'a AccountInfo<'b>,
    user: &'a AccountInfo<'b>,
    deposit_authority: &'a AccountInfo<'b>,
}

impl<'a, 'b> WithdrawMiningContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<WithdrawMiningContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let config = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let reward_pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let mining = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let user = AccountLoader::next_unchecked(account_info_iter)?;
        let deposit_authority = AccountLoader::next_signer(account_info_iter)?;

        Ok(WithdrawMiningContext {
            config,
            reward_pool,
            mining,
            user,
            deposit_authority
        })
    }

    pub fn process(&self, program_id: &Pubkey,  amount: u64) -> ProgramResult {
        let mut reward_pool = RewardPool::unpack(&self.reward_pool.data.borrow())?;
        let mut mining = Mining::unpack(&self.mining.mining.borrow())?;

        {
            let mining_pubkey = Pubkey::create_program_address(
                &[
                    b"mining".as_ref(),
                    self.user.key.as_ref(),
                    self.reward_pool.key.as_ref(),
                    &[mining.bump]
                ],
                program_id
            )?;
            assert_account_key(self.mining, &mining_pubkey)?;
            assert_account_key(self.deposit_authority, &reward_pool.deposit_authority)?;
            assert_account_key(self.config, &reward_pool.config)?;
            assert_account_key(self.reward_pool, &mining.reward_pool)?;
            assert_account_key(self.user, &mining.owner)?;
        }


        reward_pool.withdraw(&mut mining, amount)?;

        RewardPool::pack(
            reward_pool,
            *self.reward_pool.data.borrow(),
        )?;
        Mining::pack(
            mining,
            *self.mining.data.borrow(),
        )?;

        Ok(())
    }
}
