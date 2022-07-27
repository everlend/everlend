use std::str::FromStr;

use solana_program::{pubkey::Pubkey, program_pack::Pack};
use solana_program_test::{find_file, read_file, ProgramTest};
use larix_lending::state::reserve::Reserve;
use solana_sdk::account::Account;

pub const LARIX_PROGRAM_ID: &str = "7Zb1bGi32pfsrBkzWdqd4dFhUXwp5Nybr1zuaEwN34hy";
pub const LARIX_LENDING_MARKET: &str = "5geyZJdffDBNoMqEbogbPvdgH9ue7NREobtW8M3C1qfe";
pub const LARIX_RESERVE_SOL: &str = "2RcrbkGNcfy9mbarLCCRYdW3hxph7pSbP38x35MR2Bjt";
pub const LARIX_RESERVE_SOL_SUPPLY: &str = "5eSFSTPte1Hbqcvhe8H4DSgqNGuzSLjgA7ynpCucdGqg";

pub struct TestLarix {
    market_pubkey: Pubkey,
    reserve_pubkey: Pubkey,
}

pub fn add_larix(test: &mut ProgramTest) -> TestLarix {
    let market_pubkey = Pubkey::from_str(LARIX_LENDING_MARKET).unwrap();
    let reserve_pubkey = Pubkey::from_str(LARIX_RESERVE_SOL).unwrap();

    test.add_account_with_file_data(
        market_pubkey,
        u32::MAX as u64,
        larix_lending::id(),
        &format!("larix/lending_market.bin"),
    );

    let mut reserve_data = read_file(find_file("larix/reserve_sol.bin").unwrap());
    let mut reserve = Reserve::unpack_from_slice(reserve_data.as_mut_slice()).unwrap();

    test.add_account_with_file_data(
        reserve.liquidity.supply_pubkey,
        u32::MAX as u64,
        larix_lending::id(),
        "larix/liquidity_supply.bin"
    );
    test.add_account_with_file_data(
        reserve.liquidity.fee_receiver,
        u32::MAX as u64,
        larix_lending::id(),
        "larix/liquidity_fee_receiver.bin"
    );
    test.add_account_with_file_data(
        reserve.collateral.mint_pubkey,
        u32::MAX as u64,
        larix_lending::id(),
        "larix/collateral_mint.bin"
    );
    test.add_account_with_file_data(
        reserve.collateral.supply_pubkey,
        u32::MAX as u64,
        larix_lending::id(),
        "larix/collateral_supply.bin"
    );

    reserve.last_update.update_slot(0);
    Reserve::pack(reserve, &mut reserve_data).unwrap();

    test.add_account(
        reserve_pubkey,
        Account {
            lamports: u32::MAX as u64,
            data: reserve_data,
            owner: larix_lending::id(),
            executable: false,
            rent_epoch: 0,
        },
    );

    TestLarix {
        market_pubkey,
        reserve_pubkey,
    }
}
