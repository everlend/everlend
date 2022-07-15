use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum RewardsInstruction {
    I0,
    I1,
    FillVault { amount: u64 },
    DepositMining { amount: u64 },
    InitializeMining,
    WithdrawMining { amount: u64 },
    I6,
    I7,
}

pub fn fill_vault(
    program_id: &Pubkey,
    config: &Pubkey,
    reward_pool: &Pubkey,
    reward_mint: &Pubkey,
    authority: &Pubkey,
    from: &Pubkey,
    amount: u64,
) -> Instruction {
    let (vault, _) = Pubkey::find_program_address(
        &[
            b"vault".as_ref(),
            &reward_pool.to_bytes(),
            &reward_mint.to_bytes(),
        ],
        program_id,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*reward_pool, false),
        AccountMeta::new_readonly(*reward_mint, false),
        AccountMeta::new(vault, false),
        AccountMeta::new(*authority, true),
        AccountMeta::new(*from, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &RewardsInstruction::FillVault { amount },
        accounts,
    )
}

pub fn initialize_mining(
    program_id: &Pubkey,
    config: &Pubkey,
    reward_pool: &Pubkey,
    mining: &Pubkey,
    user: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*reward_pool, false),
        AccountMeta::new(*mining, false),
        AccountMeta::new_readonly(*user, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &RewardsInstruction::InitializeMining,
        accounts,
    )
}

pub fn deposit_mining(
    program_id: &Pubkey,
    config: &Pubkey,
    reward_pool: &Pubkey,
    mining: &Pubkey,
    user: &Pubkey,
    deposit_authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*reward_pool, false),
        AccountMeta::new(*mining, false),
        AccountMeta::new_readonly(*user, false),
        AccountMeta::new(*deposit_authority, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &RewardsInstruction::DepositMining { amount },
        accounts,
    )
}

pub fn withdraw_mining(
    program_id: &Pubkey,
    config: &Pubkey,
    reward_pool: &Pubkey,
    mining: &Pubkey,
    user: &Pubkey,
    deposit_authority: &Pubkey,
    amount: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*config, false),
        AccountMeta::new(*reward_pool, false),
        AccountMeta::new(*mining, false),
        AccountMeta::new_readonly(*user, false),
        AccountMeta::new(*deposit_authority, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &RewardsInstruction::WithdrawMining { amount },
        accounts,
    )
}
