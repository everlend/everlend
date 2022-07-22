use anchor_lang::InstructionData;
use eld_rewards::instruction::{InitializePool, FillVault, InitializeMining, DepositMining, WithdrawMining};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

pub fn initialize_pool(
    program_id: &Pubkey,
    config: &Pubkey,
    reward_pool: &Pubkey,
    liquidity_mint: &Pubkey,
    authority: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*config, false),
            AccountMeta::new(*reward_pool, false),
            AccountMeta::new_readonly(*liquidity_mint, false),
            AccountMeta::new_readonly(*authority, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: InitializePool.data(),
    }
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

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*config, false),
            AccountMeta::new(*reward_pool, false),
            AccountMeta::new_readonly(*reward_mint, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(*authority, true),
            AccountMeta::new(*from, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: FillVault{amount}.data()
    }
}

pub fn initialize_mining(
    program_id: &Pubkey,
    config: &Pubkey,
    reward_pool: &Pubkey,
    mining: &Pubkey,
    user: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*config, false),
            AccountMeta::new(*reward_pool, false),
            AccountMeta::new(*mining, false),
            AccountMeta::new_readonly(*user, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: InitializeMining.data(),
    }
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
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*config, false),
            AccountMeta::new_readonly(*reward_pool, false),
            AccountMeta::new(*mining, false),
            AccountMeta::new_readonly(*user, false),
            AccountMeta::new(*deposit_authority, true),
        ],
        data: DepositMining{amount}.data(),
    }
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
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*config, false),
            AccountMeta::new_readonly(*reward_pool, false),
            AccountMeta::new(*mining, false),
            AccountMeta::new_readonly(*user, false),
            AccountMeta::new(*deposit_authority, true),
        ],
        data: WithdrawMining{amount}.data(),
    }
}
