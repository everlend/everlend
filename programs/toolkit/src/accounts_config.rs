use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    io::{self, Write},
    path::Path,
    str::FromStr,
};

use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, serde_conv};
use solana_program::pubkey::Pubkey;

serde_conv!(
    PubkeyAsString,
    Pubkey,
    |pubkey: &Pubkey| pubkey.to_string(),
    |string: String| -> Result<_, std::convert::Infallible> { Ok(Pubkey::from_str(&string)?) }
);

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct DefaultAccounts {
    #[serde_as(as = "PubkeyAsString")]
    pub sol_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub usdc_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub usdt_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub msol_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub stsol_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub sobtc_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub ethw_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub ustw_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub fttw_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub ray_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub srm_mint: Pubkey,

    #[serde_as(as = "PubkeyAsString")]
    pub sol_oracle: Pubkey,

    #[serde_as(as = "PubkeyAsString")]
    pub port_finance_program_id: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub port_finance_lending_market: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub port_finance_reserve_sol: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub port_finance_reserve_sol_supply: Pubkey,

    #[serde_as(as = "PubkeyAsString")]
    pub larix_program_id: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub larix_lending_market: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub larix_reserve_sol: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub larix_reserve_sol_supply: Pubkey,

    #[serde_as(as = "PubkeyAsString")]
    pub solend_program_id: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub solend_lending_market: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub solend_reserve_sol: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub solend_reserve_sol_supply: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub solend_reserve_pyth_oracle: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub solend_reserve_switchboard_oracle: Pubkey,

    #[serde_as(as = "PubkeyAsString")]
    pub multisig_program_id: Pubkey,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub sol_collateral: Vec<Pubkey>,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub usdc_collateral: Vec<Pubkey>,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub usdt_collateral: Vec<Pubkey>,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub msol_collateral: Vec<Pubkey>,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub stsol_collateral: Vec<Pubkey>,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub sobtc_collateral: Vec<Pubkey>,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub ethw_collateral: Vec<Pubkey>,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub ustw_collateral: Vec<Pubkey>,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub fttw_collateral: Vec<Pubkey>,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub ray_collateral: Vec<Pubkey>,

    #[serde(default)]
    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub srm_collateral: Vec<Pubkey>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct InitializedAccounts {
    #[serde_as(as = "PubkeyAsString")]
    pub payer: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub registry: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub general_pool_market: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub income_pool_market: Pubkey,

    #[serde_as(as = "Vec<PubkeyAsString>")]
    pub mm_pool_markets: Vec<Pubkey>,

    #[serde(default)]
    pub token_accounts: HashMap<String, TokenAccounts>,

    #[serde_as(as = "PubkeyAsString")]
    pub liquidity_oracle: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub depositor: Pubkey,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct TokenAccounts {
    #[serde_as(as = "PubkeyAsString")]
    pub mint: Pubkey,

    #[serde_as(as = "PubkeyAsString")]
    pub liquidity_token_account: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub collateral_token_account: Pubkey,

    #[serde_as(as = "PubkeyAsString")]
    pub general_pool: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub general_pool_token_account: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub general_pool_mint: Pubkey,

    #[serde_as(as = "PubkeyAsString")]
    pub income_pool: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub income_pool_token_account: Pubkey,

    pub mm_pools: Vec<MoneyMarketAccounts>,

    #[serde_as(as = "PubkeyAsString")]
    pub liquidity_transit: Pubkey,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct MoneyMarketAccounts {
    #[serde_as(as = "PubkeyAsString")]
    pub pool: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub pool_token_account: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub token_mint: Pubkey,
    #[serde_as(as = "PubkeyAsString")]
    pub pool_mint: Pubkey,
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
        DefaultAccounts::load("default.mainnet-beta.yaml").unwrap_or_default();
    }
}
