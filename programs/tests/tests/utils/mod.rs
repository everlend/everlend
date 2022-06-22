#![allow(dead_code)]

use everlend_registry::state::{
    DistributionPubkeys, RegistryPrograms, RegistryRootAccounts, RegistrySettings,
};
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::signature::read_keypair_file;
use solana_sdk::transport;
use solana_sdk::{
    account::Account,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

pub mod depositor;
pub mod general_pool;
pub mod general_pool_borrow_authority;
pub mod general_pool_market;
pub mod income_pool;
pub mod income_pool_market;
pub mod liquidity_oracle;
pub mod money_market;
pub mod larix;
pub mod registry;
pub mod collateral_pool;
pub mod collateral_pool_borrow_authority;
pub mod collateral_pool_withdraw_authority;
pub mod collateral_pool_market;
pub mod collateral_pool_liquidity_provider;
pub mod ulp_pool;
pub mod ulp_pool_borrow_authority;
pub mod ulp_pool_market;
pub mod users;

pub use depositor::*;
pub use general_pool::*;
pub use general_pool_borrow_authority::*;
pub use general_pool_market::*;
pub use income_pool::*;
pub use income_pool_market::*;
pub use liquidity_oracle::*;
pub use money_market::*;
pub use registry::*;
pub use collateral_pool::*;
pub use collateral_pool_borrow_authority::*;
pub use collateral_pool_withdraw_authority::*;
pub use collateral_pool_market::*;
pub use ulp_pool::*;
pub use ulp_pool_borrow_authority::*;
pub use ulp_pool_market::*;
pub use users::*;

use self::larix::{add_larix, TestLarix};

pub const EXP: u64 = 1_000_000_000;
pub const REFRESH_INCOME_INTERVAL: u64 = 300; // About 2.5 min

pub type BanksClientResult<T> = transport::Result<T>;

pub struct TestEnvironment {
    pub context: ProgramTestContext,
    pub spl_token_lending: TestSPLTokenLending,
    pub pyth_oracle: TestPythOracle,
    pub registry: TestRegistry,
    pub larix: TestLarix,
}

pub fn program_test() -> ProgramTest {
    let mut program = ProgramTest::new(
        "everlend_collateral_pool",
        everlend_collateral_pool::id(),
        processor!(everlend_collateral_pool::processor::Processor::process_instruction),
    );
    program.add_program(
        "everlend_ulp",
        everlend_ulp::id(),
        processor!(everlend_ulp::processor::Processor::process_instruction),
    );
    program.add_program(
        "everlend_general_pool",
        everlend_general_pool::id(),
        processor!(everlend_general_pool::processor::Processor::process_instruction),
    );
    program.add_program(
        "everlend_income_pools",
        everlend_income_pools::id(),
        processor!(everlend_income_pools::processor::Processor::process_instruction),
    );
    program.add_program(
        "everlend_liquidity_oracle",
        everlend_liquidity_oracle::id(),
        processor!(everlend_liquidity_oracle::processor::Processor::process_instruction),
    );
    program.add_program(
        "everlend_depositor",
        everlend_depositor::id(),
        processor!(everlend_depositor::processor::Processor::process_instruction),
    );
    program.add_program(
        "everlend_registry",
        everlend_registry::id(),
        processor!(everlend_registry::processor::Processor::process_instruction),
    );
    program.add_program(
        "spl_token_lending",
        spl_token_lending::id(),
        processor!(spl_token_lending::processor::process_instruction),
    );
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

pub async fn get_mint_data(
    context: &mut ProgramTestContext,
    pubkey: &Pubkey,
) -> spl_token::state::Mint {
    let account = get_account(context, pubkey).await;
    spl_token::state::Mint::unpack_from_slice(account.data.as_slice()).unwrap()
}

pub async fn get_token_account_data(
    context: &mut ProgramTestContext,
    pubkey: &Pubkey,
) -> spl_token::state::Account {
    let account = get_account(context, pubkey).await;
    spl_token::state::Account::unpack_from_slice(account.data.as_slice()).unwrap()
}

pub async fn get_token_balance(context: &mut ProgramTestContext, pubkey: &Pubkey) -> u64 {
    let account_info = get_token_account_data(context, pubkey).await;
    account_info.amount
}

pub fn get_liquidity_mint() -> (Keypair, Pubkey) {
    let keypair = read_keypair_file("tests/fixtures/lending/liquidity.json").unwrap();
    let pubkey = keypair.pubkey();

    (keypair, pubkey)
}

pub async fn presetup() -> TestEnvironment {
    let mut test = program_test();
    let pyth_oracle = add_sol_oracle(&mut test);
    let spl_token_lending = add_spl_token_lending(&mut test);
    let larix = add_larix(&mut test);

    let mut context = test.start_with_context().await;
    let payer_pubkey = context.payer.pubkey();

    create_mint(&mut context, &get_liquidity_mint().0, &payer_pubkey)
        .await
        .unwrap();

    let registry = TestRegistry::new();
    registry.init(&mut context).await.unwrap();

    let mut programs = RegistryPrograms {
        general_pool_program_id: everlend_general_pool::id(),
        collateral_pool_program_id: everlend_collateral_pool::id(),
        liquidity_oracle_program_id: everlend_liquidity_oracle::id(),
        depositor_program_id: everlend_depositor::id(),
        income_pools_program_id: everlend_income_pools::id(),
        money_market_program_ids: DistributionPubkeys::default(),
    };
    programs.money_market_program_ids[0] = spl_token_lending::id();
    programs.money_market_program_ids[1] = larix_lending::id();

    registry
        .set_registry_config(
            &mut context,
            programs,
            RegistryRootAccounts::default(),
            RegistrySettings {
                refresh_income_interval: REFRESH_INCOME_INTERVAL,
            },
        )
        .await
        .unwrap();

    TestEnvironment {
        context,
        spl_token_lending,
        pyth_oracle,
        registry,
        larix,
    }
}

pub async fn transfer(
    context: &mut ProgramTestContext,
    pubkey: &Pubkey,
    amount: u64,
) -> BanksClientResult<()> {
    let tx = Transaction::new_signed_with_payer(
        &[system_instruction::transfer(
            &context.payer.pubkey(),
            pubkey,
            amount,
        )],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub async fn token_transfer(
    context: &mut ProgramTestContext,
    source: &Pubkey,
    destination: &Pubkey,
    authority: &Keypair,
    amount: u64,
) -> BanksClientResult<()> {
    let tx = Transaction::new_signed_with_payer(
        &[spl_token::instruction::transfer(
            &spl_token::id(),
            source,
            destination,
            &authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&context.payer.pubkey()),
        &[&context.payer, authority],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await
}

pub async fn create_token_account(
    context: &mut ProgramTestContext,
    account: &Keypair,
    mint: &Pubkey,
    manager: &Pubkey,
    lamports: u64,
) -> BanksClientResult<()> {
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN) + lamports,
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
) -> BanksClientResult<()> {
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
) -> BanksClientResult<()> {
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

pub async fn get_amount_allowed(
    context: &mut ProgramTestContext,
    test_pool: &TestPool,
    test_pool_borrow_authority: &TestPoolBorrowAuthority,
) -> u64 {
    let token_amount = get_token_balance(context, &test_pool.token_account.pubkey()).await;
    let total_amount_borrowed = test_pool.get_data(context).await.total_amount_borrowed;
    let total_pool_amount = token_amount + total_amount_borrowed;

    test_pool_borrow_authority
        .get_data(context)
        .await
        .get_amount_allowed(total_pool_amount)
        .unwrap()
}

pub async fn get_amount_allowed_general(
    context: &mut ProgramTestContext,
    test_pool: &TestGeneralPool,
    test_pool_borrow_authority: &TestGeneralPoolBorrowAuthority,
) -> u64 {
    let token_amount = get_token_balance(context, &test_pool.token_account.pubkey()).await;
    let total_amount_borrowed = test_pool.get_data(context).await.total_amount_borrowed;
    let total_pool_amount = token_amount + total_amount_borrowed;

    test_pool_borrow_authority
        .get_data(context)
        .await
        .get_amount_allowed(total_pool_amount)
        .unwrap()
}
