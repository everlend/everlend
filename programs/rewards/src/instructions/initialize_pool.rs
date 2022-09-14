use solana_program::account_info::AccountInfo;
use solana_program::entrypoint_deprecated::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_program;
use solana_program::sysvar::Sysvar;
use everlend_utils::{AccountLoader, assert_account_key};
use crate::state::{Config, InitRewardPoolParams, RewardPool};

pub struct InitializePoolContext<'a, 'b> {
    config: &'a AccountInfo<'b>,
    reward_pool: &'a AccountInfo<'b>,
    liquidity_mint: &'a AccountInfo<'b>,
    deposit_authority: &'a AccountInfo<'b>,
    payer: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitializePoolContext {
    pub fn new(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<InitializePoolContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let config = AccountLoader::next_unchecked(account_info_iter)?;
        let reward_pool = AccountLoader::next_uninitialized(account_info_iter)?;
        let liquidity_mint = AccountLoader::next_with_owner(account_info_iter, spl_token::id())?;
        let deposit_authority = AccountLoader::next_unchecked(account_info_iter)?;
        let payer = AccountLoader::next_signer(account_info_iter)?;
        let _token_program =
            AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;
        let rent = AccountLoader::next_unchecked(account_info_iter)?;

        Ok(
            InitializePoolContext {
                config,
                reward_pool,
                liquidity_mint,
                deposit_authority,
                payer,
                rent
            }
        )
    }

    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let config = Config::init(*self.deposit_authority.key);
        let rent = Rent::from_account_info(self.rent)?;

        Config::pack(
            config,
            *self.config.data.borrow(),
        )?;

        let (reward_pool_pubkey, bump) = Pubkey::find_program_address(
            &[
                "reward_pool".as_bytes(),
                self.config.key.as_ref(),
                self.liquidity_mint.key.as_ref()
            ],
            program_id
        );

        assert_account_key(self.reward_pool, &reward_pool_pubkey)?;

        let signers_seeds = &[
            "reward_pool".as_bytes(),
            self.config.key.as_ref(),
            self.liquidity_mint.key.as_ref(),
            &[bump]
        ];

        everlend_utils::cpi::system::create_account::<RewardPool>(
            program_id,
            self.payer.clone(),
            self.reward_pool.clone(),
            &[signers_seeds],
            &rent
        )?;

        let reward_pool = RewardPool::init(
            InitRewardPoolParams {
                config: *self.config.key,
                bump,
                liquidity_mint: *self.liquidity_mint.key,
                deposit_authority: *self.deposit_authority.key
            }
        );
        RewardPool::pack(
            reward_pool,
            *self.reward_pool.data.borrow(),
        )?;

        Ok(())
    }
}

// #[derive(Accounts)]
// pub struct InitializePool<'info> {
//     #[account(constraint = config.authority == payer.key())]
//     pub config: Account<'info, Config>,
//
//     /// Reward pool account
//     #[account(
//     init,
//     seeds = [
//     b"reward_pool".as_ref(),
//     config.key().as_ref(),
//     liquidity_mint.key().as_ref(),
//     ],
//     bump,
//     payer = payer,
//     space = RewardPool::LEN
//     )]
//     pub reward_pool: Account<'info, RewardPool>,
//
//     /// Mint of liquidity
//     pub liquidity_mint: Account<'info, Mint>,
//
//     /// CHECK: Authority to executes deposits on the reward pool
//     pub deposit_authority: UncheckedAccount<'info>,
//
//     #[account(mut)]
//     pub payer: Signer<'info>,
//
//     #[account(address = token::ID)]
//     pub token_program: Program<'info, Token>,
//     pub system_program: Program<'info, System>,
//     pub rent: Sysvar<'info, Rent>,
// }
//
// pub fn initialize_pool_handler(ctx: Context<InitializePool>) -> Result<()> {
//     let config = &ctx.accounts.config;
//     let reward_pool = &mut ctx.accounts.reward_pool;
//     let liquidity_mint = &ctx.accounts.liquidity_mint;
//     let deposit_authority = &ctx.accounts.deposit_authority;
//
//     reward_pool.initialize(
//         config.key(),
//         *ctx.bumps.get("reward_pool").unwrap(),
//         liquidity_mint.key(),
//         deposit_authority.key(),
//     );
//
//     emit!(InitializePoolEvent {
//         config: config.key(),
//         reward_pool: reward_pool.key(),
//         liquidity_mint: liquidity_mint.key(),
//     });
//
//     Ok(())
// }
