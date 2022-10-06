use solana_program::account_info::AccountInfo;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::{invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use borsh::BorshSerialize;

#[derive(BorshSerialize, Debug, PartialEq)]
#[repr(u8)]
pub enum ChangeKind {
    SetTo,
    ShiftBy,
}

#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    program_id: &Pubkey,
    margin_pool: AccountInfo<'a>,
    vault: AccountInfo<'a>,
    deposit_note_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    deposit_source: AccountInfo<'a>,
    deposit_account: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct DepositToLendingPool {
        instruction: [u8;8],
        change_kind: ChangeKind,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*margin_pool.key, false),
            AccountMeta::new(*vault.key, false),
            AccountMeta::new(*deposit_note_mint.key, false),
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new(*deposit_source.key, false),
            AccountMeta::new(*deposit_account.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: DepositToLendingPool {
            instruction: [242, 35, 198, 137, 82, 225, 242, 182],
            change_kind: ChangeKind::ShiftBy,
            amount,
        }
            .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            margin_pool,
            vault,
            deposit_note_mint,
            authority,
            deposit_source,
            deposit_account,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn redeem<'a>(
    program_id: &Pubkey,
    margin_pool: AccountInfo<'a>,
    vault: AccountInfo<'a>,
    deposit_note_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    source: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct WithdrawFromLendingPool {
        instruction: [ u8;8 ],
        change_kind: ChangeKind,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new(*margin_pool.key, false),
            AccountMeta::new(*vault.key, false),
            AccountMeta::new(*deposit_note_mint.key, false),
            AccountMeta::new(*source.key, false),
            AccountMeta::new(*destination.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: WithdrawFromLendingPool {
            instruction: [183, 18, 70, 156, 148, 109, 161, 34],
            change_kind: ChangeKind::ShiftBy,
            amount,
        }
            .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            authority,
            margin_pool,
            vault,
            deposit_note_mint,
            source,
            destination,
        ],
        signers_seeds,
    )
}