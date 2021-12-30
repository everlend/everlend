use crate::utils::*;
use everlend_depositor::{
    find_rebalancing_program_address, find_transit_program_address, state::Depositor,
};
use solana_client::client_error::ClientError;
use solana_program::{
    instruction::AccountMeta, program_pack::Pack, pubkey::Pubkey, system_instruction,
};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

pub fn init(
    config: &Config,
    depositor_keypair: Option<Keypair>,
    general_pool_market_pubkey: &Pubkey,
    income_pool_market_pubkey: &Pubkey,
    liquidity_oracle_pubkey: &Pubkey,
) -> Result<Pubkey, ClientError> {
    let depositor_keypair = depositor_keypair.unwrap_or_else(Keypair::new);

    println!("Depositor: {}", depositor_keypair.pubkey());

    let balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(Depositor::LEN)?;

    let tx = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &depositor_keypair.pubkey(),
                balance,
                Depositor::LEN as u64,
                &everlend_depositor::id(),
            ),
            everlend_depositor::instruction::init(
                &everlend_depositor::id(),
                &depositor_keypair.pubkey(),
                general_pool_market_pubkey,
                income_pool_market_pubkey,
                liquidity_oracle_pubkey,
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, &[&config.fee_payer, &depositor_keypair])?;

    Ok(depositor_keypair.pubkey())
}

pub fn create_transit(
    config: &Config,
    depositor_pubkey: &Pubkey,
    token_mint: &Pubkey,
) -> Result<Pubkey, ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::create_transit(
            &everlend_depositor::id(),
            depositor_pubkey,
            token_mint,
            &config.fee_payer.pubkey(),
        )],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, &[&config.fee_payer])?;

    let (transit_pubkey, _) =
        find_transit_program_address(&everlend_depositor::id(), depositor_pubkey, token_mint);

    Ok(transit_pubkey)
}

pub fn start_rebalancing(
    config: &Config,
    depositor_pubkey: &Pubkey,
    token_mint: &Pubkey,
    general_pool_market_pubkey: &Pubkey,
    general_pool_token_account: &Pubkey,
    liquidity_oracle_pubkey: &Pubkey,
) -> Result<Pubkey, ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::start_rebalancing(
            &everlend_depositor::id(),
            depositor_pubkey,
            token_mint,
            general_pool_market_pubkey,
            general_pool_token_account,
            liquidity_oracle_pubkey,
            &config.fee_payer.pubkey(),
        )],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, &[&config.fee_payer])?;

    let (rebalancing_pubkey, _) =
        find_rebalancing_program_address(&everlend_depositor::id(), depositor_pubkey, token_mint);

    Ok(rebalancing_pubkey)
}

#[allow(clippy::too_many_arguments)]
pub fn deposit(
    config: &Config,
    depositor_pubkey: &Pubkey,
    general_pool_market_pubkey: &Pubkey,
    general_pool_token_account: &Pubkey,
    mm_pool_market_pubkey: &Pubkey,
    mm_pool_token_account: &Pubkey,
    liquidity_mint: &Pubkey,
    collateral_mint: &Pubkey,
    mm_pool_collateral_mint: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            depositor_pubkey,
            general_pool_market_pubkey,
            general_pool_token_account,
            mm_pool_market_pubkey,
            mm_pool_token_account,
            mm_pool_collateral_mint,
            liquidity_mint,
            collateral_mint,
            money_market_program_id,
            money_market_accounts,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, &[&config.fee_payer])?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw(
    config: &Config,
    depositor_pubkey: &Pubkey,
    general_pool_market_pubkey: &Pubkey,
    general_pool_token_account: &Pubkey,
    income_pool_market_pubkey: &Pubkey,
    income_pool_token_account: &Pubkey,
    mm_pool_market_pubkey: &Pubkey,
    mm_pool_token_account: &Pubkey,
    collateral_mint: &Pubkey,
    liquidity_mint: &Pubkey,
    mm_pool_collateral_mint: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            depositor_pubkey,
            general_pool_market_pubkey,
            general_pool_token_account,
            income_pool_market_pubkey,
            income_pool_token_account,
            mm_pool_market_pubkey,
            mm_pool_token_account,
            mm_pool_collateral_mint,
            collateral_mint,
            liquidity_mint,
            money_market_program_id,
            money_market_accounts,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, &[&config.fee_payer])?;

    Ok(())
}
