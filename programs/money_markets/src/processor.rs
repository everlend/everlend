//! Program state processor

use everlend_depositor::instruction::MoneyMarketInstruction;
use crate::instructions::{
    DepositContext,// WithdrawContext,
};
use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

/// Program state handler.
pub struct Processor {}

impl<'a, 'b> Processor {
    /// Instruction processing router
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = MoneyMarketInstruction::try_from_slice(input)?;
        let account_info_iter = &mut accounts.iter().enumerate();

        match instruction {
            MoneyMarketInstruction::Deposit { mm, liquidity_amount } => {
                msg!("MoneyMarketInstruction: Deposit");
                DepositContext::new(program_id, account_info_iter)?.process(
                    program_id,
                    account_info_iter,
                )
            }
        }
    }
}
