use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use spl_token::state::{Account, Mint};
use everlend_utils::{AccountLoader, assert_account_key};
use everlend_utils::cpi::metaplex::program_id;

use crate::state::{RewardPool, RewardVault};

/// Instruction context
pub struct AddVaultContext<'a, 'b> {
    config: &'a AccountInfo<'b>,
    reward_pool: &'a AccountInfo<'b>,
    reward_mint: &'a AccountInfo<'b>,
    vault: &'a AccountInfo<'b>,
    fee_account: &'a AccountInfo<'b>,
    payer: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> AddVaultContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<AddVaultContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let config = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let reward_pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let reward_mint = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let vault = AccountLoader::next_unchecked(account_info_iter)?;
        let fee_account = AccountLoader::next_unchecked(account_info_iter)?;
        let payer = AccountLoader::next_signer(account_info_iter)?;
        let rent = AccountLoader::next_unchecked(account_info_iter)?;

        Ok(AddVaultContext {
            config,
            reward_pool,
            reward_mint,
            vault,
            fee_account,
            payer,
            rent
        })
    }

    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        let mut reward_pool = RewardPool::unpack(&self.reward_pool.data.borrow())?;

        let (vault_pubkey, bump) = Pubkey::find_program_address(
            &[b"vault".as_ref(), self.reward_pool.key.as_ref(), self.reward_mint.key.as_ref()],
            program_id
        );

        {
            let fee_account = Account::unpack(&self.fee_account.data.borrow())?;
            assert_account_key(self.reward_mint, &fee_account.mint)?;
            assert_account_key(self.vault, &vault_pubkey)?;
            assert_account_key(self.config, &reward_pool.config)?;
        }

        everlend_utils::cpi::spl_token::initialize_account(
            self.vault.clone(),
            self.reward_mint.clone(),
            self.reward_pool.clone(),
            self.rent.clone()
        )?;

        reward_pool.add_vault(RewardVault {
            bump,
            reward_mint: *self.reward_mint.key,
            fee_account: *self.fee_account.key,
            ..Default::default()
        })?;

        RewardPool::pack(
            reward_pool,
            *self.reward_pool.data.borrow(),
        )?;

        Ok(())
    }
}


// #[derive(Accounts)]
// pub struct AddVault<'info> {
//     #[account(constraint = config.authority == payer.key())]
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
//     /// We collect them from money market rewards
//     pub reward_mint: Account<'info, Mint>,
//
//     /// Vault for rewards
//     #[account(
//     init,
//     seeds = [b"vault".as_ref(), reward_pool.key().as_ref(), reward_mint.key().as_ref()],
//     bump,
//     payer = payer,
//     token::mint = reward_mint,
//     token::authority = reward_pool,
//     )]
//     pub vault: Account<'info, TokenAccount>,
//
//     /// Account to collect fees
//     #[account(
//     token::mint = reward_mint
//     )]
//     pub fee_account: Account<'info, TokenAccount>,
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
// pub fn add_vault_handler(ctx: Context<AddVault>) -> Result<()> {
//     let reward_pool = &mut ctx.accounts.reward_pool;
//     let reward_mint = &ctx.accounts.reward_mint;
//     let vault = &ctx.accounts.vault;
//
//     reward_pool.add_vault(RewardVault {
//         bump: *ctx.bumps.get("vault").unwrap(),
//         reward_mint: reward_mint.key(),
//         fee_account: ctx.accounts.fee_account.key(),
//         ..Default::default()
//     });
//
//     emit!(AddVaultEvent {
//         reward_pool: reward_pool.key(),
//         reward_mint: reward_mint.key(),
//         vault: vault.key(),
//     });
//
//     Ok(())
// }
