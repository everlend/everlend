use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use everlend_utils::{AccountLoader, EverlendError};
use crate::state::RewardPool;

pub struct FillVaultContext<'a, 'b> {
    config: &'a AccountInfo<'b>,
    reward_pool: &'a AccountInfo<'b>,
    reward_mint: &'a AccountInfo<'b>,
    vault: &'a AccountInfo<'b>,
    fee_account: &'a AccountInfo<'b>,
    authority: &'a AccountInfo<'b>,
    from: &'a AccountInfo<'b>,
}

impl<'a, 'b> FillVaultContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<FillVaultContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let config = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let reward_pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let reward_mint = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let vault = AccountLoader::next_unchecked(account_info_iter)?;
        let fee_account = AccountLoader::next_signer(account_info_iter)?;
        let authority = AccountLoader::next_signer(account_info_iter)?;
        let from = AccountLoader::next_signer(account_info_iter)?;

        Ok(FillVaultContext {
            config,
            reward_pool,
            reward_mint,
            vault,
            fee_account,
            authority,
            from
        })
    }

    pub fn process(&self, amount: u64) -> ProgramResult {
        let mut reward_pool = RewardPool::unpack(&self.reward_pool.data.borrow())?;

        let fee_amount = amount
            .checked_mul(FEE_PERCENTAGE)
            .ok_or(EverlendError::MathOverflow)?
            .checked_div(100)
            .ok_or(EverlendError::MathOverflow)?;
        let reward_amount = amount
            .checked_sub(fee_amount)
            .ok_or(EverlendError::MathOverflow)?;

        reward_pool.fill(*self.reward_mint.key, reward_amount)?;

        everlend_utils::cpi::spl_token::transfer(
            self.from.clone(),
            self.vault.clone(),
            self.authority.clone(),
            amount,
            &[]
        )?;

        if fee_amount > 0 {
            everlend_utils::cpi::spl_token::transfer(
                self.from.clone(),
                self.fee_account.clone(),
                self.authority.clone(),
                amount,
                &[]
            )?;
        }

        Ok(())
    }
}

// #[derive(Accounts)]
// #[instruction(amount: u64)]
// pub struct FillVault<'info> {
//     pub config: Account<'info, Config>,
//
//     /// Reward pool account
//     #[account(
//     mut,
//     has_one = config,
//
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
//     /// Account to collect fees
//     #[account(
//     mut,
//     token::mint = reward_mint,
//     address = reward_pool.vaults
//     .iter()
//     .find(|&v| v.reward_mint == reward_mint.key())
//     .ok_or(EverlendError::RewardsInvalidVault)?.fee_account
//     )]
//     pub fee_account: Account<'info, TokenAccount>,
//
//     /// Transfer authority
//     pub authority: Signer<'info>,
//
//     #[account(mut)]
//     pub from: Account<'info, TokenAccount>,
//
//     #[account(address = token::ID)]
//     pub token_program: Program<'info, Token>,
// }
//
// impl<'info> FillVault<'info> {
//     fn transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
//         CpiContext::new(
//             self.token_program.to_account_info(),
//             Transfer {
//                 from: self.from.to_account_info(),
//                 to: self.vault.to_account_info(),
//                 authority: self.authority.to_account_info(),
//             },
//         )
//     }
//
//     fn fee_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
//         CpiContext::new(
//             self.token_program.to_account_info(),
//             Transfer {
//                 from: self.from.to_account_info(),
//                 to: self.fee_account.to_account_info(),
//                 authority: self.authority.to_account_info(),
//             },
//         )
//     }
// }
//
// pub fn fill_vault_handler(ctx: Context<FillVault>, amount: u64) -> Result<()> {
//     let reward_pool = &mut ctx.accounts.reward_pool;
//     let reward_mint = &ctx.accounts.reward_mint;
//
//     let fee_amount = amount
//         .checked_mul(FEE_PERCENTAGE)
//         .ok_or(EverlendError::MathOverflow)?
//         .checked_div(100)
//         .ok_or(EverlendError::MathOverflow)?;
//
//     let reward_amount = amount.checked_sub(fee_amount).ok_or(EverlendError::MathOverflow)?;
//
//     reward_pool.fill(reward_mint.key(), reward_amount)?;
//     token::transfer(ctx.accounts.transfer_context(), reward_amount)?;
//
//     if fee_amount > 0 {
//         token::transfer(ctx.accounts.fee_transfer_context(), fee_amount)?;
//     }
//
//     Ok(())
// }
