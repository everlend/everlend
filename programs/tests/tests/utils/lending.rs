use std::str::FromStr;

use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_program_test::{find_file, read_file, ProgramTest, ProgramTestContext};
use solana_sdk::{account::Account, signature::read_keypair_file, signer::Signer};
use spl_token_lending::pyth;

use super::get_account;

pub const SOL_PYTH_PRODUCT: &str = "3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E";
pub const SOL_PYTH_PRICE: &str = "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix";

pub const SPL_LENDING_PROGRAM_ID: &str = "Bp1MJ1qr4g8t9AQJjm5H6zDB2NmRrkJL8H8zuvb1g7oV";
pub const SPL_LENDING_MARKET: &str = "CRw4TuphJ487tdTnsEKCuQkMYgGELck3Aa9Zh89xDPB";
pub const SPL_LENDING_RESERVE: &str = "8UzkB5ik7oqpcMVc4DFsj89ezD6TAiBW5GAi9W124L7F";
pub const SPL_LENDING_COLLATERL_MINT: &str = "2ph19hndAMfQ2XMGDQgdN4XjByNKAYB3acZFjHC85E6r";

pub const SOL_PRICE: i64 = 10000;

#[derive(Debug, Clone, Copy)]
pub struct TestPythOracle {
    pub product_pubkey: Pubkey,
    pub price_pubkey: Pubkey,
    pub price: i64,
}

pub fn add_pyth_oracle(
    test: &mut ProgramTest,
    product_pubkey: Pubkey,
    price_pubkey: Pubkey,
    price: i64,
) -> TestPythOracle {
    let oracle_program = read_keypair_file("tests/fixtures/pyth/program.json").unwrap();

    // Add Pyth product account
    test.add_account_with_file_data(
        product_pubkey,
        u32::MAX as u64,
        oracle_program.pubkey(),
        &format!("{}.bin", product_pubkey.to_string()),
    );

    // Add Pyth price account after setting the price
    let filename = &format!("{}.bin", price_pubkey.to_string());
    let mut pyth_price_data = read_file(find_file(filename).unwrap_or_else(|| {
        panic!("Unable to locate {}", filename);
    }));

    let mut pyth_price = pyth::load_mut::<pyth::Price>(pyth_price_data.as_mut_slice()).unwrap();

    println!("Price expo: {}", pyth_price.expo);

    pyth_price.valid_slot = 0;
    pyth_price.agg.price = price;

    test.add_account(
        price_pubkey,
        Account {
            lamports: u32::MAX as u64,
            data: pyth_price_data,
            owner: oracle_program.pubkey(),
            executable: false,
            rent_epoch: 0,
        },
    );

    TestPythOracle {
        product_pubkey,
        price_pubkey,
        price,
    }
}

pub fn add_sol_oracle(test: &mut ProgramTest) -> TestPythOracle {
    add_pyth_oracle(
        test,
        Pubkey::from_str(SOL_PYTH_PRODUCT).unwrap(),
        Pubkey::from_str(SOL_PYTH_PRICE).unwrap(),
        SOL_PRICE,
    )
}

#[derive(Debug, Clone, Copy)]
pub struct TestLending {
    pub market_pubkey: Pubkey,
    pub reserve_pubkey: Pubkey,
}

pub fn add_lending(
    test: &mut ProgramTest,
    market_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
) -> TestLending {
    // Market
    test.add_account_with_file_data(
        market_pubkey,
        u32::MAX as u64,
        spl_token_lending::id(),
        &format!("{}.bin", market_pubkey.to_string()),
    );

    // Reserve
    let filename = &format!("{}.bin", reserve_pubkey.to_string());
    let reserve_data = read_file(find_file(filename).unwrap_or_else(|| {
        panic!("Unable to locate {}", filename);
    }));
    let reserve =
        spl_token_lending::state::Reserve::unpack_from_slice(reserve_data.as_slice()).unwrap();

    test.add_account(
        reserve_pubkey,
        Account {
            lamports: u32::MAX as u64,
            data: reserve_data,
            owner: spl_token_lending::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    // Collateral mint
    test.add_account_with_file_data(
        reserve.collateral.mint_pubkey,
        u32::MAX as u64,
        spl_token::id(),
        &format!("{}.bin", reserve.collateral.mint_pubkey.to_string()),
    );

    TestLending {
        market_pubkey,
        reserve_pubkey,
    }
}

pub fn add_spl_lending(test: &mut ProgramTest) -> TestLending {
    add_lending(
        test,
        Pubkey::from_str(SPL_LENDING_MARKET).unwrap(),
        Pubkey::from_str(SPL_LENDING_RESERVE).unwrap(),
    )
}

pub async fn get_reserve_account_data(
    context: &mut ProgramTestContext,
    pubkey: &Pubkey,
) -> spl_token_lending::state::Reserve {
    let account = get_account(context, pubkey).await;
    spl_token_lending::state::Reserve::unpack_from_slice(account.data.as_slice()).unwrap()
}
