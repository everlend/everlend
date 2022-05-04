use num_enum::{FromPrimitive, IntoPrimitive};
use solana_program::{instruction::AccountMeta, pubkey::Pubkey};

pub mod larix;
pub mod solend;
pub mod spl_token_lending;

// Program IDs
pub const SPL_TOKEN_LENDING_PROGRAM_ID: &str = "Bp1MJ1qr4g8t9AQJjm5H6zDB2NmRrkJL8H8zuvb1g7oV";
// pub const PORT_FINANCE_PROGRAM_ID: &str = "pdQ2rQQU5zH2rDgZ7xH2azMBJegUzUyunJ5Jd637hC4";
// pub const LARIX_PROGRAM_ID: &str = "BDBsJpBPWtMfTgxejekYCWUAJu1mvQshiwrKuTjdEeT3";

#[derive(Debug, IntoPrimitive, FromPrimitive)]
#[repr(usize)]
pub enum MoneyMarket {
    #[num_enum(default)]
    PortFinance,
    Larix,
    Solend,
}

#[derive(Debug)]
pub enum MoneyMarketPubkeys {
    SPL(spl_token_lending::AccountPubkeys),
    PortFinance(spl_token_lending::AccountPubkeys),
    Larix(larix::AccountPubkeys),
    Solend(solend::AccountPubkeys),
}

pub fn deposit_accounts(program_id: &Pubkey, pubkeys: &MoneyMarketPubkeys) -> Vec<AccountMeta> {
    match pubkeys {
        MoneyMarketPubkeys::SPL(pubkeys) => {
            spl_token_lending::accounts::deposit(program_id, pubkeys)
        }
        MoneyMarketPubkeys::Larix(pubkeys) => larix::accounts::deposit(program_id, pubkeys),
        MoneyMarketPubkeys::Solend(pubkeys) => solend::accounts::deposit(program_id, pubkeys),
        _ => vec![],
    }
}

pub fn withdraw_accounts(program_id: &Pubkey, pubkeys: &MoneyMarketPubkeys) -> Vec<AccountMeta> {
    match pubkeys {
        MoneyMarketPubkeys::SPL(pubkeys) => {
            spl_token_lending::accounts::withdraw(program_id, pubkeys)
        }
        MoneyMarketPubkeys::Larix(pubkeys) => larix::accounts::withdraw(program_id, pubkeys),
        MoneyMarketPubkeys::Solend(pubkeys) => solend::accounts::withdraw(program_id, pubkeys),
        _ => vec![],
    }
}
