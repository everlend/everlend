use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Signature, signer::Signer, transaction::Transaction};

// pub const SUPPORTED_MINTS: &[&str] = &["SOL", "USDC", "USDT"];

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const USDC_MINT: &str = "G6YKv19AeGZ6pUYUwY9D7n4Ry9ESNFa376YqwEkUkhbi";
pub const USDT_MINT: &str = "9NGDi2tZtNmCCp8SVLKNuGjuWAVwNF3Vap5tT8km5er9";

pub const SOL_ORACLE: &str = "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix";

pub const PORT_FINANCE_LENDING_MARKET: &str = "H27Quk3DSbu55T4dCr1NddTTSAezXwHU67FPCZVKLhSW";
pub const PORT_FINANCE_RESERVE_SOL: &str = "6FeVStQAGPWvfWijDHF7cTWRCi7He6vTT3ubfNhe9SPt";
pub const PORT_FINANCE_RESERVE_SOL_SUPPLY: &str = "AbKeR7nQdHPDddiDQ71YUsz1F138a7cJMfJVtpdYUSvE";

pub const LARIX_LENDING_MARKET: &str = "HpshZh3hw9265EBSDAwopDHR5VegdEFyjxpNf9ZKH8m3";
pub const LARIX_RESERVE_SOL: &str = "j5V5dqeLGgTwackNwtmxDw9YYPZhYUBixtgh66ZKJWe";
pub const LARIX_RESERVE_SOL_SUPPLY: &str = "Ts7gZc2hhx75WSFCZdz8q1yZGGEvEBZmJRUygJhqtRh";

// Collateral tokens
pub const PORT_FINANCE_RESERVE_SOL_COLLATERAL_MINT: &str =
    "Hk4Rp3kaPssB6hnjah3Mrqpt5CAXWGoqFT5dVsWA3TaM";
pub const PORT_FINANCE_RESERVE_USDC_COLLATERAL_MINT: &str =
    "HyxraiKfdajDbYTC6MVRToEUBdevBN5M5gfyR4LC3WSF";
pub const PORT_FINANCE_RESERVE_USDT_COLLATERAL_MINT: &str =
    "4xEXmSfLFPkZaxdL98XkoxKpXEvchPVs21GYqa8DvbAm";
pub const LARIX_RESERVE_SOL_COLLATERAL_MINT: &str = "qy9PvM4J3ZdJJ7cyEFymotHrA1hTWspccA9RhDsQa24";
pub const LARIX_RESERVE_USDC_COLLATERAL_MINT: &str = "CLgRwCmZ49wbKxQjqEs5tHrNvx8ZoXjybN8hsiRwEVPm";
pub const LARIX_RESERVE_USDT_COLLATERAL_MINT: &str = "BQ4wqguD9D2dyzraCW7R4sV5fxUdXaVteT8bL2w4uPbV";

pub struct Config {
    pub rpc_client: RpcClient,
    pub verbose: bool,
    pub owner: Box<dyn Signer>,
    pub fee_payer: Box<dyn Signer>,
}

pub fn sign_and_send_and_confirm_transaction(
    config: &Config,
    mut tx: Transaction,
    signers: Vec<&dyn Signer>,
) -> Result<Signature, ClientError> {
    let recent_blockhash = config.rpc_client.get_latest_blockhash()?;

    tx.try_sign(&signers, recent_blockhash)?;

    let signature = config
        .rpc_client
        .send_and_confirm_transaction_with_spinner(&tx)?;

    Ok(signature)
}

pub fn spl_create_associated_token_account(
    config: &Config,
    wallet: &Pubkey,
    mint: &Pubkey,
) -> Result<Pubkey, ClientError> {
    let tx = Transaction::new_with_payer(
        &[
            spl_associated_token_account::create_associated_token_account(
                &config.fee_payer.pubkey(),
                wallet,
                mint,
            ),
        ],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, vec![config.fee_payer.as_ref()])?;

    let associated_token_address =
        spl_associated_token_account::get_associated_token_address(wallet, mint);

    Ok(associated_token_address)
}

pub fn spl_token_transfer(
    config: &Config,
    source_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    amount: u64,
) -> Result<(), ClientError> {
    let tx = Transaction::new_with_payer(
        &[spl_token::instruction::transfer(
            &spl_token::id(),
            source_pubkey,
            destination_pubkey,
            &config.fee_payer.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&config.fee_payer.pubkey()),
    );

    sign_and_send_and_confirm_transaction(config, tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}
