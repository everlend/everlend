use std::collections::BTreeMap;
use std::{
    fs::{create_dir_all, File},
    io::{self, Write},
    path::Path,
};

use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use solana_program::pubkey::Pubkey;

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct PortFinanceAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub program_id: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub staking_program_id: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub lending_market: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_sol: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_sol_supply: Pubkey,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct LarixAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub program_id: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub lending_market: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_sol: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_sol_supply: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub uncollateralized_ltoken_supply_sol: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub ltoken_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub mining_supply: Pubkey,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct QuarryAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub mine_program_id: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub rewarder: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub quarry: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub token_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub mint_wrapper: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub mint_wrapper_program: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub minter: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub rewards_token_mint: Pubkey,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct SolendAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub program_id: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub lending_market: Pubkey,
    // todo remove option after filling cfg file
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub reserve_pyth_oracle: Option<Pubkey>,
    // todo remove option after filling cfg file
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub reserve_switchboard_oracle: Option<Pubkey>,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_sol: Pubkey,
    // todo remove option after filling cfg file
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub reserve_sol_supply: Option<Pubkey>,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct TulipAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub program_id: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub lending_market: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_liquidity_oracle: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_liquidity_supply: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_sol: Pubkey,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct FranciumAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub program_id: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub lending_market: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_liquidity_supply: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub reserve_sol: Pubkey,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct JetAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub program_id: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub margin_pool_sol: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub vault_sol: Pubkey,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct DefaultAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub sol_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub usdc_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub usdt_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub msol_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub stsol_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub sobtc_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub ethw_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub ustw_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fttw_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub ray_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub srm_mint: Pubkey,

    #[serde_as(as = "DisplayFromStr")]
    pub sol_oracle: Pubkey,

    pub port_finance: Vec<PortFinanceAccounts>,

    pub port_accounts: BTreeMap<String, PortAccounts>,

    pub larix: Vec<LarixAccounts>,

    pub quarry: QuarryAccounts,

    pub solend: Vec<SolendAccounts>,

    pub tulip: Vec<TulipAccounts>,

    pub francium: Vec<FranciumAccounts>,

    pub jet: Vec<JetAccounts>,

    #[serde_as(as = "DisplayFromStr")]
    pub multisig_program_id: Pubkey,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub sol_collateral: Vec<Option<Pubkey>>,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub usdc_collateral: Vec<Option<Pubkey>>,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub usdt_collateral: Vec<Option<Pubkey>>,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub msol_collateral: Vec<Option<Pubkey>>,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub stsol_collateral: Vec<Option<Pubkey>>,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub sobtc_collateral: Vec<Option<Pubkey>>,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub ethw_collateral: Vec<Option<Pubkey>>,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub ustw_collateral: Vec<Option<Pubkey>>,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub fttw_collateral: Vec<Option<Pubkey>>,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub ray_collateral: Vec<Option<Pubkey>>,

    #[serde_as(as = "Vec<Option<DisplayFromStr>>")]
    pub srm_collateral: Vec<Option<Pubkey>>,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct PortAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub staking_pool: Pubkey,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct QuarryMining {
    #[serde_as(as = "DisplayFromStr")]
    pub token_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub rewards_token_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub miner_vault: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub token_source: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub rewards_token_account: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub fee_token_account: Pubkey,
}

impl QuarryMining {
    pub fn default() -> QuarryMining {
        QuarryMining {
            token_mint: Pubkey::default(),
            rewards_token_mint: Pubkey::default(),
            miner_vault: Pubkey::default(),
            token_source: Pubkey::default(),
            rewards_token_account: Pubkey::default(),
            fee_token_account: Pubkey::default(),
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct InitializedAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub payer: Pubkey,

    #[serde_as(as = "DisplayFromStr")]
    pub registry: Pubkey,

    #[serde_as(as = "DisplayFromStr")]
    pub general_pool_market: Pubkey,

    #[serde_as(as = "DisplayFromStr")]
    pub income_pool_market: Pubkey,

    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub collateral_pool_markets: Vec<Pubkey>,

    pub token_accounts: BTreeMap<String, TokenAccounts>,

    #[serde_as(as = "DisplayFromStr")]
    pub liquidity_oracle: Pubkey,

    #[serde_as(as = "DisplayFromStr")]
    pub depositor: Pubkey,

    pub quarry_mining: BTreeMap<String, QuarryMining>,
    #[serde_as(as = "DisplayFromStr")]
    pub rebalance_executor: Pubkey,

    #[serde_as(as = "DisplayFromStr")]
    pub rewards_root: Pubkey,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct TokenAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub mint: Pubkey,

    #[serde_as(as = "DisplayFromStr")]
    pub liquidity_token_account: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub collateral_token_account: Pubkey,

    #[serde_as(as = "DisplayFromStr")]
    pub general_pool: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub general_pool_token_account: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub general_pool_mint: Pubkey,

    #[serde_as(as = "DisplayFromStr")]
    pub income_pool: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub income_pool_token_account: Pubkey,

    pub mm_pools: Vec<MoneyMarketAccounts>,
    pub collateral_pools: Vec<CollateralPoolAccounts>,

    #[serde_as(as = "DisplayFromStr")]
    pub liquidity_transit: Pubkey,

    #[serde_as(as = "DisplayFromStr")]
    pub port_finance_obligation_account: Pubkey,

    pub mining_accounts: Vec<MiningAccounts>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default, Copy, Clone)]
pub struct MiningAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub staking_account: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub internal_mining_account: Pubkey,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct MoneyMarketAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub pool: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub pool_token_account: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub token_mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub pool_mint: Pubkey,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
pub struct CollateralPoolAccounts {
    #[serde_as(as = "DisplayFromStr")]
    pub pool: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub pool_token_account: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub token_mint: Pubkey,
}

impl InitializedAccounts {
    pub fn load(config_file: &str) -> Result<Self, io::Error> {
        load_config_file(config_file)
    }

    pub fn save(&self, config_file: &str) -> Result<(), io::Error> {
        save_config_file(self, config_file)
    }
}

impl DefaultAccounts {
    pub fn load(config_file: &str) -> Result<Self, io::Error> {
        load_config_file(config_file)
    }
}

pub fn load_config_file<T, P>(config_file: P) -> Result<T, io::Error>
where
    T: serde::de::DeserializeOwned,
    P: AsRef<Path>,
{
    let file = File::open(config_file)?;
    let config = serde_yaml::from_reader(file)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{:?}", err)))?;
    Ok(config)
}

pub fn save_config_file<T, P>(config: &T, config_file: P) -> Result<(), io::Error>
where
    T: serde::ser::Serialize,
    P: AsRef<Path>,
{
    let serialized = serde_yaml::to_string(config)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("{:?}", err)))?;

    if let Some(outdir) = config_file.as_ref().parent() {
        create_dir_all(outdir)?;
    }
    let mut file = File::create(config_file)?;
    file.write_all(&serialized.into_bytes())?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_accounts_load() {
        DefaultAccounts::load("default.devnet.yaml").unwrap();
    }

    #[test]
    fn default_accounts_load_save() {
        let cfg_name = "accounts.devnet.yaml";
        let a = InitializedAccounts::load(cfg_name).unwrap();
        a.save(cfg_name).unwrap();
        let b = InitializedAccounts::load(cfg_name).unwrap();
        assert_eq!(a, b);
    }
}
