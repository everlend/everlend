use solana_program::instruction::InstructionError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
use solana_program::system_instruction::SystemError;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};
use spl_token::error::TokenError;

use everlend_income_pools::instruction;
use everlend_income_pools::state::AccountType;
use everlend_utils::EverlendError;

use crate::utils::*;

async fn setup() -> (ProgramTestContext, TestIncomePoolMarket) {
    let (mut context, _, _, registry) = presetup().await;

    let general_pool_market = TestGeneralPoolMarket::new();
    general_pool_market.init(&mut context, &registry.keypair.pubkey()).await.unwrap();

    let test_income_pool_market = TestIncomePoolMarket::new();
    test_income_pool_market
        .init(&mut context, &general_pool_market)
        .await
        .unwrap();

    (context, test_income_pool_market)
}

#[tokio::test]
async fn success() {
    let (mut context, test_income_pool_market) = setup().await;

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);
    test_income_pool
        .create(&mut context, &test_income_pool_market)
        .await
        .unwrap();

    let pool = test_income_pool.get_data(&mut context).await;

    assert_eq!(pool.account_type, AccountType::IncomePool);
}

#[tokio::test]
async fn fail_second_time_create() {
    let (mut context, test_income_pool_market) = setup().await;

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);
    test_income_pool
        .create(&mut context, &test_income_pool_market)
        .await
        .unwrap();

    let pool = test_income_pool.get_data(&mut context).await;

    assert_eq!(pool.account_type, AccountType::IncomePool);

    let tx = Transaction::new_signed_with_payer(
        &[instruction::create_pool(
            &everlend_income_pools::id(),
            &test_income_pool_market.keypair.pubkey(),
            &test_income_pool.token_mint_pubkey,
            &test_income_pool.token_account.pubkey(),
            &test_income_pool_market.manager.pubkey(),
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer, &test_income_pool_market.manager],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(SystemError::AccountAlreadyInUse as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_wrong_account_owner() {
    let (mut context, test_income_pool_market) = setup().await;

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);

    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &test_income_pool.token_account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                &Pubkey::new_unique(),
            ),
            instruction::create_pool(
                &everlend_income_pools::id(),
                &test_income_pool_market.keypair.pubkey(),
                &test_income_pool.token_mint_pubkey,
                &test_income_pool.token_account.pubkey(),
                &test_income_pool_market.manager.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &test_income_pool_market.manager,
            &test_income_pool.token_account,
        ],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(1, InstructionError::ExternalAccountDataModified)
    );
}

#[tokio::test]
async fn fail_with_invalid_pool_market() {
    let (mut context, test_income_pool_market) = setup().await;

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);

    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &test_income_pool.token_account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                &everlend_income_pools::id(),
            ),
            instruction::create_pool(
                &everlend_income_pools::id(),
                &Pubkey::new_unique(),
                &test_income_pool.token_mint_pubkey,
                &test_income_pool.token_account.pubkey(),
                &test_income_pool_market.manager.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &test_income_pool_market.manager,
            &test_income_pool.token_account,
        ],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(EverlendError::InvalidAccountOwner as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_token_mint() {
    let (mut context, test_income_pool_market) = setup().await;

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);

    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &test_income_pool.token_account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                &everlend_income_pools::id(),
            ),
            instruction::create_pool(
                &everlend_income_pools::id(),
                &test_income_pool_market.keypair.pubkey(),
                &Pubkey::new_unique(),
                &test_income_pool.token_account.pubkey(),
                &test_income_pool_market.manager.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &test_income_pool_market.manager,
            &test_income_pool.token_account,
        ],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            1,
            InstructionError::Custom(TokenError::InvalidMint as u32),
        )
    );
}

#[tokio::test]
async fn fail_with_invalid_token_account() {
    let (mut context, test_income_pool_market) = setup().await;

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);

    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &test_income_pool.token_account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                &everlend_income_pools::id(),
            ),
            instruction::create_pool(
                &everlend_income_pools::id(),
                &test_income_pool_market.keypair.pubkey(),
                &test_income_pool.token_mint_pubkey,
                &Pubkey::new_unique(),
                &test_income_pool_market.manager.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &test_income_pool_market.manager,
            &test_income_pool.token_account,
        ],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(1, InstructionError::InvalidAccountData)
    );
}

#[tokio::test]
async fn fail_with_wrong_manager() {
    let (mut context, test_income_pool_market) = setup().await;

    let test_income_pool = TestIncomePool::new(&test_income_pool_market, None);

    let rent = context.banks_client.get_rent().await.unwrap();

    let wrong_manager = Keypair::new();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &test_income_pool.token_account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                &everlend_income_pools::id(),
            ),
            instruction::create_pool(
                &everlend_income_pools::id(),
                &test_income_pool_market.keypair.pubkey(),
                &test_income_pool.token_mint_pubkey,
                &test_income_pool.token_account.pubkey(),
                &wrong_manager.pubkey(),
            ),
        ],
        Some(&context.payer.pubkey()),
        &[
            &context.payer,
            &wrong_manager,
            &test_income_pool.token_account,
        ],
        context.last_blockhash,
    );

    assert_eq!(
        context
            .banks_client
            .process_transaction(tx)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(1, InstructionError::InvalidArgument)
    );
}
