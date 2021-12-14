use super::get_account;
use solana_program::{clock::Slot, program_pack::Pack, pubkey::Pubkey};
use solana_program_test::{find_file, read_file, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::{Account, AccountSharedData},
    signature::read_keypair_file,
    signer::Signer,
};
use spl_token_lending::pyth;
use std::str::FromStr;

pub const SOL_PYTH_PRODUCT: &str = "3Mnn2fX6rQyUsyELYms1sBJyChWofzSNRoqYzvgMVz5E";
pub const SOL_PYTH_PRICE: &str = "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix";

pub const SPL_TOKEN_LENDING_MARKET: &str = "JEEQ6mvMvzvcuVtBjNhTFb7yNdQSKybVTsFxEhMGfRjK";
pub const SPL_TOKEN_LENDING_RESERVE: &str = "4LKaeb5dEipZjBF9UzkiDCLJjpfPBokkTa2VD9LMwBem";

pub const SOL_PRICE: i64 = 10000;

#[derive(Debug, Clone, Copy)]
pub struct TestPythOracle {
    pub product_pubkey: Pubkey,
    pub price_pubkey: Pubkey,
    pub price: i64,
}

impl TestPythOracle {
    pub async fn update(&self, context: &mut ProgramTestContext, slot: Slot) {
        let mut account = get_account(context, &self.price_pubkey).await;
        let mut pyth_price = pyth::load_mut::<pyth::Price>(account.data.as_mut_slice()).unwrap();

        pyth_price.valid_slot = slot;

        context.set_account(&self.price_pubkey, &AccountSharedData::from(account));
    }
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
pub struct TestSPLTokenLending {
    pub market_pubkey: Pubkey,
    pub reserve_pubkey: Pubkey,
}

impl TestSPLTokenLending {
    pub async fn get_reserve_data(
        &self,
        context: &mut ProgramTestContext,
    ) -> spl_token_lending::state::Reserve {
        let account = get_account(context, &self.reserve_pubkey).await;
        spl_token_lending::state::Reserve::unpack_from_slice(account.data.as_slice()).unwrap()
    }
}

pub fn add_spl_token_lending(test: &mut ProgramTest) -> TestSPLTokenLending {
    let market_pubkey = Pubkey::from_str(SPL_TOKEN_LENDING_MARKET).unwrap();
    let reserve_pubkey = Pubkey::from_str(SPL_TOKEN_LENDING_RESERVE).unwrap();

    // Add market
    test.add_account_with_file_data(
        market_pubkey,
        u32::MAX as u64,
        spl_token_lending::id(),
        &format!("{}.bin", market_pubkey.to_string()),
    );

    // Reserve
    let filename = &format!("{}.bin", reserve_pubkey.to_string());
    let mut reserve_data = read_file(find_file(filename).unwrap_or_else(|| {
        panic!("Unable to locate {}", filename);
    }));
    let mut reserve =
        spl_token_lending::state::Reserve::unpack_from_slice(reserve_data.as_mut_slice()).unwrap();

    // Add sub token accounts
    test.add_account_with_file_data(
        reserve.liquidity.supply_pubkey,
        u32::MAX as u64,
        spl_token::id(),
        &format!("{}.bin", reserve.liquidity.supply_pubkey.to_string()),
    );
    test.add_account_with_file_data(
        reserve.liquidity.fee_receiver,
        u32::MAX as u64,
        spl_token::id(),
        &format!("{}.bin", reserve.liquidity.fee_receiver.to_string()),
    );
    test.add_account_with_file_data(
        reserve.collateral.mint_pubkey,
        u32::MAX as u64,
        spl_token::id(),
        &format!("{}.bin", reserve.collateral.mint_pubkey.to_string()),
    );
    test.add_account_with_file_data(
        reserve.collateral.supply_pubkey,
        u32::MAX as u64,
        spl_token::id(),
        &format!("{}.bin", reserve.collateral.supply_pubkey.to_string()),
    );

    reserve.last_update.update_slot(0);
    spl_token_lending::state::Reserve::pack(reserve, &mut reserve_data).unwrap();

    // Add reserve
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

    TestSPLTokenLending {
        market_pubkey,
        reserve_pubkey,
    }
}
