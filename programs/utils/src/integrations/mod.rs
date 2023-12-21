use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use num_enum::{FromPrimitive, IntoPrimitive};
use solana_program::{instruction::AccountMeta, pubkey::Pubkey};

pub mod frakt;
pub mod francium;
pub mod jet;
pub mod larix;
pub mod solend;
pub mod spl_token_lending;
pub mod tulip;

// Program IDs
pub const SPL_TOKEN_LENDING_PROGRAM_ID: &str = "Bp1MJ1qr4g8t9AQJjm5H6zDB2NmRrkJL8H8zuvb1g7oV";
// pub const PORT_FINANCE_PROGRAM_ID: &str = "pdQ2rQQU5zH2rDgZ7xH2azMBJegUzUyunJ5Jd637hC4";
// pub const LARIX_PROGRAM_ID: &str = "BDBsJpBPWtMfTgxejekYCWUAJu1mvQshiwrKuTjdEeT3";

#[derive(Debug, BorshSchema, BorshDeserialize, BorshSerialize, PartialEq, Clone, Copy)]
pub enum MoneyMarket {
    PortFinance {
        money_market_program_id: Pubkey,
    },
    Larix {
        money_market_program_id: Pubkey,
    },
    Solend {
        money_market_program_id: Pubkey,
        lending_market: Pubkey,
    },
    Tulip {
        money_market_program_id: Pubkey,
    },
    Francium {
        money_market_program_id: Pubkey,
    },
    Jet {
        money_market_program_id: Pubkey,
    },
    Frakt {
        money_market_program_id: Pubkey,
        liquidity_pool: Pubkey,
    },
}

impl MoneyMarket {
    pub fn program_id(&self) -> Pubkey {
        match self {
            MoneyMarket::PortFinance {
                money_market_program_id,
            } => *money_market_program_id,
            MoneyMarket::Larix {
                money_market_program_id,
            } => *money_market_program_id,
            MoneyMarket::Solend {
                money_market_program_id,
                ..
            } => *money_market_program_id,
            MoneyMarket::Tulip {
                money_market_program_id,
            } => *money_market_program_id,
            MoneyMarket::Francium {
                money_market_program_id,
            } => *money_market_program_id,
            MoneyMarket::Jet {
                money_market_program_id,
            } => *money_market_program_id,
            MoneyMarket::Frakt {
                money_market_program_id,
                ..
            } => *money_market_program_id,
        }
    }

    // num_enum doesn't work for non-unit enums
    pub fn num(&self) -> usize {
        match self {
            MoneyMarket::PortFinance { .. } => 0,
            MoneyMarket::Larix { .. } => 1,
            MoneyMarket::Solend { .. } => 2,
            MoneyMarket::Tulip { .. } => 3,
            MoneyMarket::Francium { .. } => 4,
            MoneyMarket::Jet { .. } => 5,
            MoneyMarket::Frakt { .. } => 6,
        }
    }
}

impl Default for MoneyMarket {
    fn default() -> Self {
        Self::PortFinance {
            money_market_program_id: Pubkey::default(),
        }
    }
}

#[derive(Debug, IntoPrimitive, FromPrimitive, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum StakingMoneyMarket {
    #[num_enum(default)]
    None,
    PortFinance,
    Larix,
    Solend,
    Quarry,
    Francium,
}

#[derive(Debug)]
pub enum MoneyMarketPubkeys {
    SPL(spl_token_lending::AccountPubkeys),
    PortFinance(spl_token_lending::AccountPubkeys),
    Larix(larix::AccountPubkeys),
    Solend(solend::AccountPubkeys),
    Tulip(tulip::AccountPubkeys),
    Francium(francium::AccountPubkeys),
    Jet(jet::AccountPubkeys),
    Frakt(frakt::AccountPubkeys),
}

pub fn deposit_accounts(program_id: &Pubkey, pubkeys: &MoneyMarketPubkeys) -> Vec<AccountMeta> {
    match pubkeys {
        MoneyMarketPubkeys::SPL(pubkeys) => {
            spl_token_lending::accounts::deposit(program_id, pubkeys)
        }
        MoneyMarketPubkeys::Larix(pubkeys) => larix::accounts::deposit(program_id, pubkeys),
        MoneyMarketPubkeys::Solend(pubkeys) => solend::accounts::deposit(program_id, pubkeys),
        MoneyMarketPubkeys::Tulip(pubkeys) => tulip::accounts::deposit(program_id, pubkeys),
        MoneyMarketPubkeys::Francium(pubkeys) => francium::accounts::deposit(program_id, pubkeys),
        MoneyMarketPubkeys::Jet(pubkeys) => jet::accounts::deposit(program_id, pubkeys),
        MoneyMarketPubkeys::Frakt(pubkeys) => frakt::accounts::deposit(program_id, pubkeys),
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
        MoneyMarketPubkeys::Tulip(pubkeys) => tulip::accounts::withdraw(program_id, pubkeys),
        MoneyMarketPubkeys::Francium(pubkeys) => francium::accounts::withdraw(program_id, pubkeys),
        MoneyMarketPubkeys::Jet(pubkeys) => jet::accounts::withdraw(program_id, pubkeys),
        MoneyMarketPubkeys::Frakt(pubkeys) => frakt::accounts::withdraw(program_id, pubkeys),
        _ => vec![],
    }
}
