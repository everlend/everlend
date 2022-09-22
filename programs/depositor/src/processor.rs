//! Program state processor

use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

use crate::instruction::DepositorInstruction;
use crate::instructions::{
    ClaimMiningRewardsContext, CreateTransitContext, DepositContext, InitContext,
    InitMiningAccountContext, MigrateDepositorContext, RefreshMMIncomesContext,
    SetRebalancingContext, StartRebalancingContext, WithdrawContext,
};

/// Program state handler.
pub struct Processor {}

impl<'a, 'b> Processor {
    /// Instruction processing router
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = DepositorInstruction::try_from_slice(input)?;
        let account_info_iter = &mut accounts.iter().enumerate();

        match instruction {
            DepositorInstruction::Init { rebalance_executor } => {
                msg!("DepositorInstruction: Init");
                InitContext::new(program_id, account_info_iter)?.process(
                    program_id,
                    account_info_iter,
                    rebalance_executor,
                )
            }

            DepositorInstruction::CreateTransit { seed } => {
                msg!("DepositorInstruction: CreateTransit");
                CreateTransitContext::new(program_id, account_info_iter)?.process(
                    program_id,
                    account_info_iter,
                    seed,
                )
            }

            DepositorInstruction::StartRebalancing { refresh_income } => {
                msg!("DepositorInstruction: StartRebalancing");
                StartRebalancingContext::new(program_id, account_info_iter)?.process(
                    program_id,
                    account_info_iter,
                    refresh_income,
                )
            }

            DepositorInstruction::SetRebalancing {
                amount_to_distribute,
                distributed_liquidity,
                distribution_array,
            } => {
                msg!("DepositorInstruction: ResetRebalancing");
                SetRebalancingContext::new(program_id, account_info_iter)?.process(
                    program_id,
                    account_info_iter,
                    amount_to_distribute,
                    distributed_liquidity,
                    distribution_array,
                )
            }

            DepositorInstruction::Deposit => {
                msg!("DepositorInstruction: Deposit");
                DepositContext::new(program_id, account_info_iter)?
                    .process(program_id, account_info_iter)
            }

            DepositorInstruction::Withdraw => {
                msg!("DepositorInstruction: Withdraw");
                WithdrawContext::new(program_id, account_info_iter)?
                    .process(program_id, account_info_iter)
            }

            DepositorInstruction::MigrateDepositor => {
                msg!("DepositorInstruction: MigrateDepositor");
                MigrateDepositorContext::new(program_id, account_info_iter)?
                    .process(program_id, account_info_iter)
            }

            DepositorInstruction::InitMiningAccount { mining_type } => {
                msg!("DepositorInstruction: InitMiningAccount");
                InitMiningAccountContext::new(program_id, account_info_iter)?.process(
                    program_id,
                    account_info_iter,
                    mining_type,
                )
            }

            DepositorInstruction::ClaimMiningReward { with_subrewards } => {
                msg!("DepositorInstruction: ClaimMiningReward");
                ClaimMiningRewardsContext::new(program_id, account_info_iter)?.process(
                    program_id,
                    account_info_iter,
                    with_subrewards,
                )
            }

            DepositorInstruction::RefreshMMIncomes => {
                msg!("DepositorInstruction: RefreshMMIncomes");
                RefreshMMIncomesContext::new(program_id, account_info_iter)?
                    .process(program_id, account_info_iter)
            }
        }
    }
}
