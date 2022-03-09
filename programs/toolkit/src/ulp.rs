use crate::utils::*;
use everlend_ulp::{find_pool_program_address, instruction, state::PoolMarket};
use solana_client::client_error::ClientError;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_sdk::{
    signature::{write_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

pub fn create_market(
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
                &everlend_ulp::id(),
            ),
            // Initialize pool market account
            instruction::init_pool_market(
                &everlend_ulp::id(),
                &pool_market_keypair.pubkey(),
                &config.fee_payer.pubkey(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(
        config,
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

pub fn create_pool(
    config: &Config,
    pool_market_pubkey: &Pubkey,
    token_mint: &Pubkey,
) -> Result<(Pubkey, Pubkey, Pubkey), ClientError> {
    // Generate new accounts
    let token_account = Keypair::new();
    let pool_mint = Keypair::new();

    let (pool_pubkey, _) =
        find_pool_program_address(&everlend_ulp::id(), pool_market_pubkey, token_mint);

    println!("Pool: {}", &pool_pubkey);
    println!("Token account: {}", &token_account.pubkey());
    println!("Pool mint: {}", &pool_mint.pubkey());

    let account_info = config
        .rpc_client
        .get_account_with_commitment(&pool_pubkey, config.rpc_client.commitment())?
        .value;
    if account_info.is_some() {
        return Ok((pool_pubkey, token_account.pubkey(), pool_mint.pubkey()));
    }

    let token_account_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;
    let pool_mint_balance = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)?;

    let tx = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &token_account.pubkey(),
                token_account_balance,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            system_instruction::create_account(
                &config.fee_payer.pubkey(),
                &pool_mint.pubkey(),
                pool_mint_balance,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            instruction::create_pool(
                &everlend_ulp::id(),
                pool_market_pubkey,
                token_mint,
                &token_account.pubkey(),
                &pool_mint.pubkey(),
                &config.fee_payer.pubkey(),
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(
        config,
        tx,
        vec![config.fee_payer.as_ref(), &token_account, &pool_mint],
    )?;

    Ok((pool_pubkey, token_account.pubkey(), pool_mint.pubkey()))
}
