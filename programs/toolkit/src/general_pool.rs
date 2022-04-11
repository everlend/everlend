use crate::utils::*;
use everlend_general_pool::{
    find_pool_borrow_authority_program_address, find_pool_program_address,
    find_user_withdrawal_request_program_address, find_withdrawal_requests_program_address,
    instruction,
    state::{PoolMarket, WithdrawalRequest, WithdrawalRequests},
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
                &everlend_general_pool::id(),
            ),
            // Initialize pool market account
            instruction::init_pool_market(
                &everlend_general_pool::id(),
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
        find_pool_program_address(&everlend_general_pool::id(), pool_market_pubkey, token_mint);

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
                &everlend_general_pool::id(),
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

pub fn create_pool_borrow_authority(
    config: &Config,
    pool_market_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    borrow_authority: &Pubkey,
    share_allowed: u16,
) -> Result<Pubkey, ClientError> {
    let (pool_borrow_authority_pubkey, _) = find_pool_borrow_authority_program_address(
        &everlend_general_pool::id(),
        pool_pubkey,
        borrow_authority,
    );

    println!("Pool borrow authority: {}", &pool_borrow_authority_pubkey);

    let account_info = config
        .rpc_client
        .get_account_with_commitment(
            &pool_borrow_authority_pubkey,
            config.rpc_client.commitment(),
        )?
        .value;
    if account_info.is_some() {
        return Ok(pool_borrow_authority_pubkey);
    }

    let tx = Transaction::new_with_payer(
        &[instruction::create_pool_borrow_authority(
            &everlend_general_pool::id(),
            pool_market_pubkey,
            pool_pubkey,
            borrow_authority,
            &config.fee_payer.pubkey(),
            share_allowed,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, vec![config.fee_payer.as_ref()])?;

    Ok(pool_borrow_authority_pubkey)
}

#[allow(clippy::too_many_arguments)]
pub fn deposit(
    config: &Config,
    pool_market_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    source: &Pubkey,
    destination: &Pubkey,
    pool_token_account: &Pubkey,
    pool_mint: &Pubkey,
    amount: u64,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[instruction::deposit(
            &everlend_general_pool::id(),
            pool_market_pubkey,
            pool_pubkey,
            source,
            destination,
            pool_token_account,
            pool_mint,
            &config.fee_payer.pubkey(),
            amount,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_request(
    config: &Config,
    pool_market_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    source: &Pubkey,
    destination: &Pubkey,
    pool_token_account: &Pubkey,
    token_mint: &Pubkey,
    pool_mint: &Pubkey,
    amount: u64,
    index: u64,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[instruction::withdraw_request(
            &everlend_general_pool::id(),
            pool_market_pubkey,
            pool_pubkey,
            source,
            destination,
            pool_token_account,
            token_mint,
            pool_mint,
            &config.fee_payer.pubkey(),
            amount,
            index,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw(
    config: &Config,
    pool_market_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    destination: &Pubkey,
    pool_token_account: &Pubkey,
    token_mint: &Pubkey,
    pool_mint: &Pubkey,
    index: u64,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[instruction::withdraw(
            &everlend_general_pool::id(),
            pool_market_pubkey,
            pool_pubkey,
            destination,
            pool_token_account,
            token_mint,
            pool_mint,
            &config.fee_payer.pubkey(),
            index,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn cancel_withdraw_request(
    config: &Config,
    pool_market_pubkey: &Pubkey,
    pool_pubkey: &Pubkey,
    source: &Pubkey,
    token_mint: &Pubkey,
    pool_mint: &Pubkey,
    rent_payer: &Pubkey,
    index: u64,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[instruction::cancel_withdraw_request(
            &everlend_general_pool::id(),
            pool_market_pubkey,
            pool_pubkey,
            source,
            token_mint,
            pool_mint,
            &config.fee_payer.pubkey(),
            rent_payer,
            index,
        )],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}

pub fn get_general_pool_market(
    config: &Config,
    pool_market_pubkey: &Pubkey,
) -> Result<PoolMarket, ClientError> {
    let account = config.rpc_client.get_account(&pool_market_pubkey)?;
    Ok(PoolMarket::unpack(&account.data).unwrap())
}

pub fn get_withdraw_requests(
    config: &Config,
    pool_market_pubkey: &Pubkey,
    token_mint: &Pubkey,
) -> Result<(Pubkey, WithdrawalRequests), ClientError> {
    let (withdrawal_requests_pubkey, _) = find_withdrawal_requests_program_address(
        &everlend_general_pool::id(),
        pool_market_pubkey,
        token_mint,
    );

    let withdrawal_requests_account = config.rpc_client.get_account(&withdrawal_requests_pubkey)?;
    let withdrawal_requests =
        WithdrawalRequests::unpack(&withdrawal_requests_account.data).unwrap();

    Ok((withdrawal_requests_pubkey, withdrawal_requests))
}

pub fn get_withdraw_request(
    config: &Config,
    pool_market_pubkey: &Pubkey,
    token_mint: &Pubkey,
    index: u64,
) -> Result<(Pubkey, WithdrawalRequest), ClientError> {
    let (withdrawal_request_pubkey, _) = find_user_withdrawal_request_program_address(
        &everlend_general_pool::id(),
        pool_market_pubkey,
        token_mint,
        index,
    );
    let withdrawal_request_account = config.rpc_client.get_account(&withdrawal_request_pubkey)?;
    let withdrawal_request = WithdrawalRequest::unpack(&withdrawal_request_account.data).unwrap();

    Ok((withdrawal_request_pubkey, withdrawal_request))
}

pub fn current_withdrawal_request_index(
    config: &Config,
    pool_market_pubkey: &Pubkey,
    token_mint: &Pubkey,
) -> Result<u64, ClientError> {
    let (withdrawal_requests_pubkey, _) = find_withdrawal_requests_program_address(
        &everlend_general_pool::id(),
        pool_market_pubkey,
        token_mint,
    );

    let withdrawal_requests_account = config.rpc_client.get_account(&withdrawal_requests_pubkey)?;
    let withdrawal_requests =
        WithdrawalRequests::unpack(&withdrawal_requests_account.data).unwrap();

    Ok(withdrawal_requests.last_request_id + 1)
}
