use everlend_depositor::{
    find_rebalancing_program_address, find_transit_program_address,
    state::{Depositor, Rebalancing},
};
use everlend_liquidity_oracle::state::DistributionArray;
use solana_client::client_error::ClientError;
use solana_program::{
    instruction::AccountMeta, program_pack::Pack, pubkey::Pubkey, system_instruction,
};
use solana_sdk::{
    signature::{write_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

use crate::utils::*;

pub fn init_depositor(
    config: &Config,
    registry_pubkey: &Pubkey,
    depositor_keypair: Option<Keypair>,
    rebalance_executor: Pubkey,
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
                registry_pubkey,
                &depositor_keypair.pubkey(),
                &rebalance_executor,
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), &depositor_keypair],
    )?;

    write_keypair_file(
        &depositor_keypair,
        &format!(".keypairs/{}.json", depositor_keypair.pubkey()),
    )
    .unwrap();

    Ok(depositor_keypair.pubkey())
}

pub fn create_transit(
    config: &Config,
    depositor_pubkey: &Pubkey,
    token_mint: &Pubkey,
    seed: Option<String>,
) -> Result<Pubkey, ClientError> {
    let (transit_pubkey, _) = find_transit_program_address(
        &everlend_depositor::id(),
        depositor_pubkey,
        token_mint,
        &seed.clone().unwrap_or_default(),
    );

    let account_info = config
        .rpc_client
        .get_account_with_commitment(&transit_pubkey, config.rpc_client.commitment())?
        .value;
    if account_info.is_some() {
        return Ok(transit_pubkey);
    }

    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::create_transit(
            &everlend_depositor::id(),
            depositor_pubkey,
            token_mint,
            &config.fee_payer.pubkey(),
            seed,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(transit_pubkey)
}

#[allow(clippy::too_many_arguments)]
pub fn start_rebalancing(
    config: &Config,
    registry_pubkey: &Pubkey,
    depositor_pubkey: &Pubkey,
    token_mint: &Pubkey,
    general_pool_market_pubkey: &Pubkey,
    general_pool_token_account: &Pubkey,
    liquidity_oracle_pubkey: &Pubkey,
    refresh_income: bool,
) -> Result<(Pubkey, Rebalancing), ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::start_rebalancing(
            &everlend_depositor::id(),
            registry_pubkey,
            depositor_pubkey,
            token_mint,
            general_pool_market_pubkey,
            general_pool_token_account,
            liquidity_oracle_pubkey,
            &config.fee_payer.pubkey(),
            refresh_income,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    let (rebalancing_pubkey, _) =
        find_rebalancing_program_address(&everlend_depositor::id(), depositor_pubkey, token_mint);

    let rebalancing_account = config.rpc_client.get_account(&rebalancing_pubkey)?;
    let rebalancing = Rebalancing::unpack(&rebalancing_account.data).unwrap();

    Ok((rebalancing_pubkey, rebalancing))
}

pub fn reset_rebalancing(
    config: &Config,
    registry_pubkey: &Pubkey,
    depositor_pubkey: &Pubkey,
    token_mint: &Pubkey,
    amount_to_distribute: u64,
    distributed_liquidity: u64,
    distribution_array: DistributionArray,
) -> Result<(Pubkey, Rebalancing), ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::reset_rebalancing(
            &everlend_depositor::id(),
            registry_pubkey,
            depositor_pubkey,
            token_mint,
            &config.fee_payer.pubkey(),
            amount_to_distribute,
            distributed_liquidity,
            distribution_array,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    let (rebalancing_pubkey, _) =
        find_rebalancing_program_address(&everlend_depositor::id(), depositor_pubkey, token_mint);

    let rebalancing_account = config.rpc_client.get_account(&rebalancing_pubkey)?;
    let rebalancing = Rebalancing::unpack(&rebalancing_account.data).unwrap();

    Ok((rebalancing_pubkey, rebalancing))
}

#[allow(clippy::too_many_arguments)]
pub fn depositor_deposit(
    config: &Config,
    registry_pubkey: &Pubkey,
    depositor_pubkey: &Pubkey,
    liquidity_mint: &Pubkey,
    collateral_mint: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
    collateral_storage_deposit_accounts: Vec<AccountMeta>,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::deposit(
            &everlend_depositor::id(),
            registry_pubkey,
            depositor_pubkey,
            liquidity_mint,
            collateral_mint,
            &config.fee_payer.pubkey(),
            money_market_program_id,
            money_market_accounts,
            collateral_storage_deposit_accounts,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn depositor_withdraw(
    config: &Config,
    registry_pubkey: &Pubkey,
    depositor_pubkey: &Pubkey,
    income_pool_market_pubkey: &Pubkey,
    income_pool_token_account: &Pubkey,
    collateral_mint: &Pubkey,
    liquidity_mint: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
    collateral_storage_accounts: Vec<AccountMeta>,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::withdraw(
            &everlend_depositor::id(),
            registry_pubkey,
            depositor_pubkey,
            income_pool_market_pubkey,
            income_pool_token_account,
            collateral_mint,
            liquidity_mint,
            &config.fee_payer.pubkey(),
            money_market_program_id,
            money_market_accounts,
            collateral_storage_accounts,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

pub fn migrate_depositor(
    config: &Config,
    depositor: &Pubkey,
    registry: &Pubkey,
    liquidity_mint: &Pubkey,
    amount_to_distribute: u64,
) -> Result<(), ClientError> {
    let (rebalancing, _) =
        find_rebalancing_program_address(&everlend_depositor::id(), depositor, liquidity_mint);

    let tx = Transaction::new_with_payer(
        &[everlend_depositor::instruction::migrate_depositor(
            &everlend_depositor::id(),
            depositor,
            registry,
            &config.fee_payer.pubkey(),
            &rebalancing,
            liquidity_mint,
            amount_to_distribute,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    let depositor: Depositor = config.get_account_unpack(depositor)?;
    println!("Migration of Depositor finished: \n{:?}", &depositor);

    Ok(())
}
