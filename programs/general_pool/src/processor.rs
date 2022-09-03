//! Program processor
use borsh::BorshDeserialize;
use everlend_utils::EverlendError;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

use crate::instruction::LiquidityPoolsInstruction;
use crate::instructions::{
    CreatePoolBorrowAuthorityContext, CreatePoolContext, DeletePoolBorrowAuthorityContext,
    DepositContext, InitPoolMarketContext, SetPoolConfigContext, UpdatePoolBorrowAuthorityContext,
};

/// Instruction processing router
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = LiquidityPoolsInstruction::try_from_slice(input)?;

    match instruction {
        LiquidityPoolsInstruction::InitPoolMarket => {
            msg!("LiquidityPoolsInstruction: InitPoolMarket");
            InitPoolMarketContext::new(program_id, accounts)?.process(program_id)
        }

        LiquidityPoolsInstruction::CreatePool => {
            msg!("LiquidityPoolsInstruction: CreatePool");
            CreatePoolContext::new(program_id, accounts)?.process(program_id)
        }

        LiquidityPoolsInstruction::CreatePoolBorrowAuthority { share_allowed } => {
            msg!("LiquidityPoolsInstruction: CreatePoolBorrowAuthority");
            CreatePoolBorrowAuthorityContext::new(program_id, accounts)?
                .process(program_id, share_allowed)
        }

        LiquidityPoolsInstruction::UpdatePoolBorrowAuthority { share_allowed } => {
            msg!("LiquidityPoolsInstruction: UpdatePoolBorrowAuthority");
            UpdatePoolBorrowAuthorityContext::new(program_id, accounts)?
                .process(program_id, share_allowed)
        }

        LiquidityPoolsInstruction::DeletePoolBorrowAuthority => {
            msg!("LiquidityPoolsInstruction: DeletePoolBorrowAuthority");
            DeletePoolBorrowAuthorityContext::new(program_id, accounts)?.process(program_id)
        }

        LiquidityPoolsInstruction::Deposit { amount } => {
            msg!("LiquidityPoolsInstruction: Deposit");
            DepositContext::new(program_id, accounts)?.process(program_id, amount)
        }

        LiquidityPoolsInstruction::Withdraw => {
            msg!("LiquidityPoolsInstruction: Withdraw");
            WithdrawContext::new(program_id, accounts)?.process(program_id)
        }

        LiquidityPoolsInstruction::WithdrawRequest { collateral_amount } => {
            msg!("LiquidityPoolsInstruction: WithdrawRequest");
            WithdrawRequestContext::new(program_id, accounts)?
                .process(program_id, collateral_amount)
        }

        LiquidityPoolsInstruction::CancelWithdrawRequest => {
            msg!("LiquidityPoolsInstruction: CancelWithdrawRequest");
            CancelWithdrawRequestContext::new(program_id, accounts)?.process(program_id)
        }

        LiquidityPoolsInstruction::Borrow { amount } => {
            msg!("LiquidityPoolsInstruction: Borrow");
            BorrowContext::new(program_id, accounts)?.process(program_id, amount)
        }

        LiquidityPoolsInstruction::Repay {
            amount,
            interest_amount,
        } => {
            msg!("LiquidityPoolsInstruction: Repay");
            CreatePoolContext::new(program_id, accounts)?.process(
                program_id,
                amount,
                interest_amount,
            )
        }

        LiquidityPoolsInstruction::ClosePoolMarket => {
            msg!("LiquidityPoolsInstruction: ClosePoolMarket");
            Err(EverlendError::TemporaryUnavailable.into())
        }

        LiquidityPoolsInstruction::MigrationInstruction => {
            msg!("LiquidityPoolsInstruction: MigrationInstruction");
            Err(EverlendError::TemporaryUnavailable.into())
        }

        LiquidityPoolsInstruction::InitUserMining => {
            msg!("LiquidityPoolsInstruction: InitUserMining");
            InitUserMiningContext::new(program_id, accounts)?.process(program_id)
        }

        LiquidityPoolsInstruction::UpdateManager => {
            msg!("LiquidityPoolsInstruction: UpdateManager");
            UpdateManagerContext::new(program_id, accounts)?.process(program_id)
        }

        LiquidityPoolsInstruction::SetTokenMetadata { name, symbol, uri } => {
            msg!("LiquidityPoolsInstruction: SetTokenMetadata");
            SetTokenMetadataContext::new(program_id, accounts)?
                .process(program_id, name, symbol, uri)
        }

        LiquidityPoolsInstruction::SetPoolConfig { params } => {
            msg!("LiquidityPoolsInstruction: SetPoolConfig");
            SetPoolConfigContext::new(program_id, accounts)?.process(program_id, params)
        }
    }
}
