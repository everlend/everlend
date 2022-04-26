use solana_client::client_error::ClientError;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_sdk::{
    signature::{write_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

use everlend_liquidity_oracle::state::TokenDistribution;
use everlend_liquidity_oracle::{
    find_liquidity_oracle_token_distribution_program_address, instruction,
    state::{DistributionArray, LiquidityOracle},
};

use crate::utils::*;

pub fn init(config: &Config, oracle_keypair: Option<Keypair>) -> Result<Pubkey, ClientError> {
    let oracle_keypair = oracle_keypair.unwrap_or_else(Keypair::new);

    println!("Liquidity oracle: {}", oracle_keypair.pubkey());

    let balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(LiquidityOracle::LEN)?;

    let tx = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &oracle_keypair.pubkey(),
                balance,
                LiquidityOracle::LEN as u64,
                &everlend_liquidity_oracle::id(),
            ),
            instruction::init_liquidity_oracle(
                &everlend_liquidity_oracle::id(),
                &oracle_keypair.pubkey(),
                &config.fee_payer.pubkey(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), &oracle_keypair],
    )?;

    write_keypair_file(
        &oracle_keypair,
        &format!(".keypairs/{}.json", oracle_keypair.pubkey()),
    )
    .unwrap();

    Ok(oracle_keypair.pubkey())
}

pub fn create_token_distribution(
    config: &Config,
    oracle_pubkey: &Pubkey,
    token_mint: &Pubkey,
    distribution: &DistributionArray,
) -> Result<Pubkey, ClientError> {
    let (token_distribution_pubkey, _) = find_liquidity_oracle_token_distribution_program_address(
        &everlend_liquidity_oracle::id(),
        oracle_pubkey,
        token_mint,
    );

    let account_info = config
        .rpc_client
        .get_account_with_commitment(&token_distribution_pubkey, config.rpc_client.commitment())?
        .value;
    if account_info.is_some() {
        return Ok(token_distribution_pubkey);
    }

    let tx = Transaction::new_with_payer(
        &[instruction::create_token_distribution(
            &everlend_liquidity_oracle::id(),
            oracle_pubkey,
            &config.fee_payer.pubkey(),
            token_mint,
            *distribution,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(token_distribution_pubkey)
}

pub fn update_token_distribution(
    config: &Config,
    oracle_pubkey: &Pubkey,
    token_mint: &Pubkey,
    distribution: &DistributionArray,
) -> Result<Pubkey, ClientError> {
    let tx = Transaction::new_with_payer(
        &[instruction::update_token_distribution(
            &everlend_liquidity_oracle::id(),
            oracle_pubkey,
            &config.fee_payer.pubkey(),
            token_mint,
            *distribution,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    let (token_distribution_pubkey, _) = find_liquidity_oracle_token_distribution_program_address(
        &everlend_liquidity_oracle::id(),
        oracle_pubkey,
        token_mint,
    );

    Ok(token_distribution_pubkey)
}

fn migrate(config: &Config, token_mint: Pubkey) -> Result<(), ClientError> {
    let accounts = config.get_initialized_accounts();

    let liqduidty_mint = &token_mint;

    println!("Sending migration itx ...");
    let tx = Transaction::new_with_payer(
        &[instruction::migrate(
            &everlend_liquidity_oracle::id(),
            &accounts.liquidity_oracle,
            &config.fee_payer.pubkey(),
            &liqduidty_mint,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    let (new_acc_key, _) = find_liquidity_oracle_token_distribution_program_address(
        &everlend_depositor::id(),
        &accounts.liquidity_oracle,
        &liqduidty_mint,
    );

    let new: TokenDistribution = config.get_account_unpack(&new_acc_key)?;

    println!("Migration of TokenDistribution successfully: \n{:?}", new);

    Ok(())
}

pub fn migrate_wrapper(config: &Config, token_mint: Option<&str>) -> Result<(), anyhow::Error> {
    let default_accounts = config.get_default_accounts();

    let (mint_map, _) = get_asset_maps(default_accounts);

    if token_mint.is_none() {
        mint_map
            .iter()
            .map(|mint| {
                println!("Migration for {}: {}", mint.0, mint.1);
                migrate(config, *mint.1)
            })
            .collect::<Result<Vec<()>, ClientError>>()?;
        print!("Migration completed for all mints");
    } else {
        let mint_name = token_mint.unwrap();
        let mint = mint_map
            .get(mint_name)
            .expect(&format!("Mint not found for token: {}", mint_name));
        println!("Migration for {}: {}", mint_name, mint);
        migrate(config, *mint)?;
    }
    Ok(())
}
