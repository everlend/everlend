use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use everlend_utils::{AccountLoader, assert_account_key, EverlendError};
use crate::state::{Mining, RewardPool};

pub struct ClaimContext<'a, 'b> {
    root_account: &'a AccountInfo<'b>,
    reward_pool: &'a AccountInfo<'b>,
    reward_mint: &'a AccountInfo<'b>,
    vault: &'a AccountInfo<'b>,
    mining: &'a AccountInfo<'b>,
    user: &'a AccountInfo<'b>,
    user_reward_token_account: &'a AccountInfo<'b>,
}

impl<'a, 'b> ClaimContext<'a, 'b> {
    pub fn new(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<ClaimContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let root_account = AccountLoader::next_unchecked(account_info_iter)?;
        let reward_pool = AccountLoader::next_uninitialized(account_info_iter)?;
        let reward_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let vault = AccountLoader::next_unchecked(account_info_iter)?;
        let mining = AccountLoader::next_unchecked(account_info_iter)?;
        let user = AccountLoader::next_signer(account_info_iter)?;
        let user_reward_token_account = AccountLoader::next_unchecked(account_info_iter)?;
        let _token_program =
            AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        Ok(
            ClaimContext {
                root_account,
                reward_pool,
                reward_mint,
                vault,
                mining,
                user,
                user_reward_token_account
            }
        )
    }

    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let reward_pool = RewardPool::unpack(&self.reward_pool.data.borrow())?;
        let mut mining = Mining::unpack(&self.mining.data.borrow())?;

        let reward_pool_seeds = &[
            b"reward_pool".as_ref(),
            &reward_pool.root_account.to_bytes()[..32],
            &reward_pool.liquidity_mint.to_bytes()[..32],
            &[reward_pool.bump],
        ];

        {
            assert_account_key(self.root_account, &reward_pool.root_account)?;
            assert_account_key(self.reward_pool, &mining.reward_pool)?;
            assert_account_key(
                self.reward_pool,
                &Pubkey::create_program_address(
                    reward_pool_seeds,
                    program_id
                )?
            )?;

            let bump = reward_pool.vaults.iter().find(|v| {
                &v.reward_mint == self.reward_mint.key
            }).ok_or(EverlendError::MathOverflow)?.bump;// todo make error
            let vault_seeds = &[
                b"vault".as_ref(),
                &self.reward_pool.key.to_bytes()[..32],
                &self.reward_mint.key.to_bytes()[..32],
                &[bump]
            ];
            assert_account_key(self.vault, &Pubkey::create_program_address(
                vault_seeds,
                program_id,
            )?)?;
        }

        mining.refresh_rewards(reward_pool.vaults.iter())?;

        let mut reward_index = mining.reward_index_mut(*self.reward_mint.key);
        let amount = reward_index.rewards;

        reward_index.rewards = 0;

        everlend_utils::cpi::spl_token::transfer(
            self.vault.clone(),
            self.user_reward_token_account.clone(),
            self.reward_pool.clone(),
            amount,
            &[reward_pool_seeds]
        )?;

        Mining::pack(
            mining,
            *self.reward_pool.data.borrow_mut()
        )?;
        RewardPool::pack(
            reward_pool,
            *self.reward_pool.data.borrow_mut()
        )?;

        Ok(())
    }
}

// #[derive(Accounts)]
// pub struct Claim<'info> {
//     pub config: Account<'info, Config>,
//
//     /// Reward pool account
//     #[account(
//     mut,
//     has_one = config,
//     )]
//     pub reward_pool: Account<'info, RewardPool>,
//
//     /// Mint of rewards
//     pub reward_mint: Account<'info, Mint>,
//
//     /// Vault for rewards
//     #[account(
//     mut,
//     seeds = [b"vault".as_ref(), reward_pool.key().as_ref(), reward_mint.key().as_ref()],
//     bump = reward_pool.vaults
//     .iter()
//     .find(|&v| v.reward_mint == reward_mint.key())
//     .ok_or(EverlendError::RewardsInvalidVault)?.bump
//     )]
//     pub vault: Account<'info, TokenAccount>,
//
//     /// Mining
//     #[account(
//     mut,
//     seeds = [
//     b"mining".as_ref(),
//     user.key().as_ref(),
//     reward_pool.key().as_ref(),
//     ],
//     bump = mining.bump,
//     has_one = reward_pool,
//     constraint = mining.owner == user.key(),
//     )]
//     pub mining: Account<'info, Mining>,
//
//     #[account(mut)]
//     pub user: Signer<'info>,
//
//     /// User token account for reward
//     #[account(mut)]
//     pub user_reward_token_account: Account<'info, TokenAccount>,
//
//     #[account(address = token::ID)]
//     pub token_program: Program<'info, Token>,
// }
//
// impl<'info> Claim<'info> {
//     fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
//         CpiContext::new(
//             self.token_program.to_account_info(),
//             Transfer {
//                 from: self.vault.to_account_info(),
//                 to: self.user_reward_token_account.to_account_info(),
//                 authority: self.reward_pool.to_account_info(),
//             },
//         )
//     }
// }
//
// pub fn claim_handler(ctx: Context<Claim>) -> Result<()> {
//     let reward_pool =  &ctx.accounts.reward_pool;
//     let mining = &mut ctx.accounts.mining;
//
//     mining.refresh_rewards(reward_pool.vaults.iter())?;
//
//     let reward_mint = &ctx.accounts.reward_mint;
//
//     let mut reward_index = mining.reward_index_mut(reward_mint.key());
//     let amount = reward_index.rewards;
//
//     reward_index.rewards = 0;
//
//     // Reward pool authority seeds
//     let seeds = &[
//         b"reward_pool".as_ref(),
//         &reward_pool.config.to_bytes()[..32],
//         &reward_pool.liquidity_mint.to_bytes()[..32],
//         &[ctx.accounts.reward_pool.bump],
//     ];
//
//     token::transfer(
//         ctx.accounts.transfer_context().with_signer(&[seeds]),
//         amount,
//     )?;
//
//     Ok(())
// }
