use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use spl_token::state::Mint;
use everlend_utils::{AccountLoader, assert_account_key};
use crate::state::{Mining, RewardPool};

pub struct InitializeMiningContext<'a, 'b> {
    config: &'a AccountInfo<'b>,
    reward_pool: &'a AccountInfo<'b>,
    mining: &'a AccountInfo<'b>,
    user: &'a AccountInfo<'b>,
    payer: &'a AccountInfo<'b>,
    deposit_authority: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitializeMiningContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<InitializeMiningContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let config = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let reward_pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let mining = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let user = AccountLoader::next_unchecked(account_info_iter)?;
        let deposit_authority = AccountLoader::next_unchecked(account_info_iter)?;
        let payer = AccountLoader::next_signer(account_info_iter)?;
        let rent = AccountLoader::next_unchecked(account_info_iter)?;

        Ok(InitializeMiningContext {
            config,
            reward_pool,
            mining,
            user,
            payer,
            deposit_authority,
            rent
        })
    }

    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let reward_pool = RewardPool::unpack(&self.reward_pool.data.borrow())?;
        let rent = Rent::from_account_info(self.rent)?;

        let (mining_pubkey, mining_bump) = Pubkey::find_program_address(
            &[
                b"mining".as_ref(),
                user.key().as_ref(),
                reward_pool.key().as_ref()
            ],
            program_id
        );

        assert_account_key(self.mining, &mining_pubkey)?;

        everlend_utils::cpi::system::create_account::<Mining>(
            program_id,
            self.user.clone(),
            self.mining.clone(),
            &[],
            &rent,
        )?;

        let mining = Mining::initialize(
            reward_pool.key(),
            mining_bump,
            *self.user.key,
        );
        Mining::pack(
            mining,
            *self.mining.data.borrow()
        )?;

        Ok(())
    }
}

// #[derive(Accounts)]
// pub struct InitializeMining<'info> {
//     pub config: Account<'info, Config>,
//
//     /// Reward pool account
//     #[account(
//     mut,
//     has_one = config,
//     )]
//     pub reward_pool: Account<'info, RewardPool>,
//
//     /// Mining account
//     #[account(
//     init,
//     seeds = [
//     b"mining".as_ref(),
//     user.key().as_ref(),
//     reward_pool.key().as_ref(),
//     ],
//     bump,
//     payer = payer,
//     space = Mining::LEN,
//     )]
//     pub mining: Account<'info, Mining>,
//
//     /// CHECK: Owner of mining account
//     pub user: UncheckedAccount<'info>,
//
//     #[account(mut)]
//     pub payer: Signer<'info>,
//
//     pub system_program: Program<'info, System>,
//     pub rent: Sysvar<'info, Rent>,
// }
//
// pub fn initialize_mining_handler(ctx: Context<InitializeMining>) -> Result<()> {
//     let reward_pool = &ctx.accounts.reward_pool;
//     let mining = &mut ctx.accounts.mining;
//     let user = &ctx.accounts.user;
//
//     mining.initialize(
//         reward_pool.key(),
//         *ctx.bumps.get("mining").unwrap(),
//         user.key(),
//     );
//
//     Ok(())
// }
