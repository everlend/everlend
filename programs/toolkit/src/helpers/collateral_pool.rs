use crate::utils::*;
use anchor_lang::prelude::AccountMeta;
use everlend_collateral_pool::state::{PoolBorrowAuthority, PoolWithdrawAuthority};
use everlend_collateral_pool::{
    find_pool_borrow_authority_program_address, find_pool_program_address,
    find_pool_withdraw_authority_program_address,
    instruction::{self, CollateralPoolsInstruction},
    state::{Pool, PoolMarket},
};
use solana_client::client_error::ClientError;
use solana_program::{
    instruction::Instruction, program_pack::Pack, pubkey::Pubkey, system_instruction,
    system_program, sysvar,
};
use solana_sdk::{
    signature::{write_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

pub struct PoolPubkeys {
    pub pool: Pubkey,
    pub token_account: Pubkey,
}

pub fn create_collateral_market(
    config: &Config,
    pool_market_keypair: Option<Keypair>,
) -> Result<Pubkey, ClientError> {
    let pool_market_keypair = pool_market_keypair.unwrap_or_else(Keypair::new);

    println!("Pool market: {}", pool_market_keypair.pubkey());

    let balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(PoolMarket::LEN)?;

    let tx = Transaction::new_with_payer(
        &[
            // Pool market account
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &pool_market_keypair.pubkey(),
                balance,
                PoolMarket::LEN as u64,
                &everlend_collateral_pool::id(),
            ),
            // Initialize pool market account
            instruction::init_pool_market(
                &everlend_collateral_pool::id(),
                &pool_market_keypair.pubkey(),
                &config.fee_payer.pubkey(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), &pool_market_keypair],
    )?;

    write_keypair_file(
        &pool_market_keypair,
        &format!(".keypairs/{}.json", pool_market_keypair.pubkey()),
    )
    .unwrap();

    Ok(pool_market_keypair.pubkey())
}

pub fn create_collateral_pool(
    config: &Config,
    pool_market_pubkey: &Pubkey,
    token_mint: &Pubkey,
) -> Result<PoolPubkeys, ClientError> {
    let (pool_pubkey, _) = find_pool_program_address(
        &everlend_collateral_pool::id(),
        pool_market_pubkey,
        token_mint,
    );

    let account_info = config
        .rpc_client
        .get_account_with_commitment(&pool_pubkey, config.rpc_client.commitment())?
        .value;
    if account_info.is_some() {
        let pool = config.get_account_unpack::<Pool>(&pool_pubkey)?;
        return Ok(PoolPubkeys {
            pool: pool_pubkey,
            token_account: pool.token_account,
        });
    }

    let token_account = Keypair::new();

    println!("Pool: {}", &pool_pubkey);
    println!("Token account: {}", &token_account.pubkey());

    let token_account_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;
    let tx = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &token_account.pubkey(),
                token_account_balance,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            everlend_collateral_pool::instruction::create_pool(
                &everlend_collateral_pool::id(),
                pool_market_pubkey,
                token_mint,
                &token_account.pubkey(),
                &config.fee_payer.pubkey(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), &token_account],
    )?;

    Ok(PoolPubkeys {
        pool: pool_pubkey,
        token_account: token_account.pubkey(),
    })
}

pub fn create_pool_withdraw_authority(
    config: &Config,
    pool_market: &Pubkey,
    pool: &Pubkey,
    withdraw_authority: &Pubkey,
    manager: &Pubkey,
) -> Result<Pubkey, ClientError> {
    let (pool_withdraw_authority, _) = find_pool_withdraw_authority_program_address(
        &everlend_collateral_pool::id(),
        pool,
        withdraw_authority,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(*pool, false),
        AccountMeta::new(pool_withdraw_authority, false),
        AccountMeta::new_readonly(*withdraw_authority, false),
        AccountMeta::new(*manager, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = Instruction::new_with_borsh(
        everlend_collateral_pool::id(),
        &CollateralPoolsInstruction::CreatePoolWithdrawAuthority,
        accounts,
    );
    let tx = Transaction::new_with_payer(&[instruction], Some(&config.fee_payer.pubkey()));

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;
    println!(
        "Pool: {} Withdraw authority: {}",
        pool, pool_withdraw_authority
    );
    Ok(pool_withdraw_authority)
}

pub fn bulk_migrate_pool_withdraw_authority(
    config: &Config,
    pool_withdraw_authority: &[(Pubkey, Pubkey, PoolWithdrawAuthority)],
) -> Result<(), ClientError> {
    let instructions: Vec<Instruction> = pool_withdraw_authority
        .iter()
        .map(|(market_pubkey, authority_pubkey, authority)| {
            let (new_pool_withdraw_authority, _) = find_pool_withdraw_authority_program_address(
                &everlend_collateral_pool::id(),
                &authority.pool,
                &authority.withdraw_authority,
            );

            let accounts = vec![
                AccountMeta::new_readonly(*market_pubkey, false),
                AccountMeta::new_readonly(authority.pool, false),
                AccountMeta::new(*authority_pubkey, false),
                AccountMeta::new(new_pool_withdraw_authority, false),
                AccountMeta::new(config.fee_payer.pubkey(), true),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ];

            Instruction::new_with_borsh(
                everlend_collateral_pool::id(),
                &CollateralPoolsInstruction::MigratePoolWithdrawAuthority,
                accounts,
            )
        })
        .collect();

    let tx = Transaction::new_with_payer(&instructions, Some(&config.fee_payer.pubkey()));

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

pub fn bulk_migrate_pool_borrow_authority(
    config: &Config,
    pool_borrow_authority: &[(Pubkey, Pubkey, PoolBorrowAuthority)],
) -> Result<(), ClientError> {
    let instructions: Vec<Instruction> = pool_borrow_authority
        .iter()
        .map(|(market_pubkey, authority_pubkey, authority)| {
            let (new_pool_borrow_authority, _) = find_pool_borrow_authority_program_address(
                &everlend_collateral_pool::id(),
                &authority.pool,
                &authority.borrow_authority,
            );

            let accounts = vec![
                AccountMeta::new_readonly(*market_pubkey, false),
                AccountMeta::new_readonly(authority.pool, false),
                AccountMeta::new(*authority_pubkey, false),
                AccountMeta::new(new_pool_borrow_authority, false),
                AccountMeta::new(config.fee_payer.pubkey(), true),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ];

            Instruction::new_with_borsh(
                everlend_collateral_pool::id(),
                &CollateralPoolsInstruction::MigratePoolBorrowAuthority,
                accounts,
            )
        })
        .collect();

    let tx = Transaction::new_with_payer(&instructions, Some(&config.fee_payer.pubkey()));

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

pub fn collateral_pool_update_manager(
    config: &Config,
    pool_market: &Pubkey,
    manager: &Keypair,
    new_manager: &Keypair,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[instruction::update_manager(
            &everlend_collateral_pool::id(),
            pool_market,
            &manager.pubkey(),
            &new_manager.pubkey(),
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), manager, new_manager],
    )?;

    Ok(())
}
