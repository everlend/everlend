//! Program state processor

use crate::{
    instruction::DepositorInstruction,
    utils::{assert_owned_by, check_deposit, ulp_borrow},
};
use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Process Deposit instruction
    pub fn deposit(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let ulp_pool_market_info = next_account_info(account_info_iter)?;
        let ulp_pool_info = next_account_info(account_info_iter)?;
        let ulp_pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let ulp_token_account_info = next_account_info(account_info_iter)?;
        let staging_token_account_info = next_account_info(account_info_iter)?;
        let depositor_info = next_account_info(account_info_iter)?;
        let instructions_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        if !depositor_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Borrow from ULP to staging account
        ulp_borrow(
            ulp_pool_market_info.clone(),
            ulp_pool_info.clone(),
            ulp_pool_borrow_authority_info.clone(),
            staging_token_account_info.clone(),
            ulp_token_account_info.clone(),
            depositor_info.clone(),
            amount,
            &[],
        )?;

        check_deposit(instructions_info, amount)?;

        // ...

        Ok(())
    }

    /// Instruction processing router
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = DepositorInstruction::try_from_slice(input)?;

        match instruction {
            DepositorInstruction::Deposit { amount } => {
                msg!("DepositorInstruction: Deposit");
                Self::deposit(program_id, amount, accounts)
            }
        }
    }
}
