use crate::utils::*;
use everlend_income_pools::{
    instruction,
    state::{IncomePool, IncomePoolMarket},
};
use solana_client::client_error::ClientError;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_sdk::{
    signature::{write_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

pub fn create_market(
    config: &Config,
    income_pool_market_keypair: Option<Keypair>,
    general_pool_market_pubkey: &Pubkey,
) -> Result<Pubkey, ClientError> {
    let income_pool_market_keypair = income_pool_market_keypair.unwrap_or_else(Keypair::new);

    println!(
        "Income pool market: {}",
        income_pool_market_keypair.pubkey()
    );

    let balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(IncomePoolMarket::LEN)?;

    let tx = Transaction::new_with_payer(
        &[
            // Income pool market account
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &income_pool_market_keypair.pubkey(),
                balance,
                IncomePoolMarket::LEN as u64,
                &everlend_income_pools::id(),
            ),
            // Initialize income pool market account
            instruction::init_pool_market(
                &everlend_income_pools::id(),
                &income_pool_market_keypair.pubkey(),
                &config.fee_payer.pubkey(),
                general_pool_market_pubkey,
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), &income_pool_market_keypair],
    )?;

    write_keypair_file(
        &income_pool_market_keypair,
        &format!(".keypairs/{}.json", income_pool_market_keypair.pubkey()),
    )
    .unwrap();

    Ok(income_pool_market_keypair.pubkey())
}

pub fn create_pool(
    config: &Config,
    income_pool_market_pubkey: &Pubkey,
    token_mint: &Pubkey,
) -> Result<(Pubkey, Pubkey), ClientError> {
    let (income_pool_pubkey, _) = everlend_income_pools::find_pool_program_address(
        &everlend_income_pools::id(),
        income_pool_market_pubkey,
        token_mint,
    );
    
    let account_info = config
        .rpc_client
        .get_account_with_commitment(&income_pool_pubkey, config.rpc_client.commitment())?
        .value;
    if account_info.is_some() {
        let income_pool = config.get_account_unpack::<IncomePool>(&income_pool_pubkey)?;
        return Ok((income_pool_pubkey, income_pool.token_account));
    }

    // Generate new accounts
    let token_account = Keypair::new();

    println!("Income pool: {}", &income_pool_pubkey);
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
            instruction::create_pool(
                &everlend_income_pools::id(),
                income_pool_market_pubkey,
                token_mint,
                &token_account.pubkey(),
                &config.fee_payer.pubkey(),
            ),
            instruction::safety_pool_token_account(
                &everlend_income_pools::id(),
                token_mint,
                income_pool_market_pubkey,
                &income_pool_pubkey,
                &config.fee_payer.pubkey(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), &token_account],
    )?;

    Ok((income_pool_pubkey, token_account.pubkey()))
}

pub fn create_income_pool_safety_fund_token_account(
    config: &Config,
    income_pool_market_pubkey: &Pubkey,
    token_mint: &Pubkey,
) -> Result<(), ClientError> {
    let (income_pool_pubkey, _) = everlend_income_pools::find_pool_program_address(
        &everlend_income_pools::id(),
        income_pool_market_pubkey,
        token_mint,
    );

    let tx = Transaction::new_with_payer(
        &[
            instruction::safety_pool_token_account(
                &everlend_income_pools::id(),
                token_mint,
                income_pool_market_pubkey,
                &income_pool_pubkey,
                &config.fee_payer.pubkey(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}
