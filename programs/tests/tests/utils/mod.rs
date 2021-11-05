#![allow(dead_code)]

use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport,
};

pub mod pool;
pub mod pool_borrow_authority;
pub mod pool_market;
pub mod depositor;
pub mod users;

pub use pool::*;
pub use pool_borrow_authority::*;
pub use pool_market::*;
pub use depositor::*;
pub use users::*;

pub const EXP: u64 = 1_000_000_000;

pub fn program_test() -> ProgramTest {
    let mut program = ProgramTest::new(
        "everlend_ulp",
        everlend_ulp::id(),
        processor!(everlend_ulp::processor::Processor::process_instruction),
    );
    program.add_program(
        "everlend_depositor",
        everlend_depositor::id(),
        processor!(everlend_depositor::processor::Processor::process_instruction),
    );
    // program.add_program(
    //     "spl_token_lending",
    //     spl_token_lending::id(),
    //     processor!(spl_token_lending::processor::Processor::process_instruction),
    // );
    program
}

pub async fn get_account(context: &mut ProgramTestContext, pubkey: &Pubkey) -> Account {
    context
        .banks_client
        .get_account(*pubkey)
        .await
        .expect("account not found")
        .expect("account empty")
}

pub async fn get_token_balance(context: &mut ProgramTestContext, pubkey: &Pubkey) -> u64 {
    let account = get_account(context, pubkey).await;
    let account_info: spl_token::state::Account =
        spl_token::state::Account::unpack_from_slice(account.data.as_slice()).unwrap();

    account_info.amount
}

pub async fn create_token_account(
    context: &mut ProgramTestContext,
    account: &Keypair,
    mint: &Pubkey,
    manager: &Pubkey,
) -> transport::Result<()> {
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &account.pubkey(),
                mint,
                manager,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, account],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub async fn create_mint(
    context: &mut ProgramTestContext,
    mint: &Keypair,
    manager: &Pubkey,
) -> transport::Result<()> {
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(spl_token::state::Mint::LEN),
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                manager,
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, mint],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub async fn mint_tokens(
    context: &mut ProgramTestContext,
    mint: &Pubkey,
    account: &Pubkey,
    amount: u64,
) -> transport::Result<()> {
    let tx = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            account,
            &context.payer.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}
