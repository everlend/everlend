use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Signature, signer::Signer, transaction::Transaction};

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

pub const SOL_ORACLE: &str = "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix";
pub const SOL_LARIX_ORACLE: &str = "6bGUz6bdWAvaUf6PuEdvdAxrWbZ9wF5XkAevDsEKsb7y";

pub const PORT_FINANCE_LENDING_MARKET: &str = "H27Quk3DSbu55T4dCr1NddTTSAezXwHU67FPCZVKLhSW";
pub const PORT_FINANCE_RESERVE_SOL: &str = "6FeVStQAGPWvfWijDHF7cTWRCi7He6vTT3ubfNhe9SPt";
pub const PORT_FINANCE_RESERVE_SOL_SUPPLY: &str = "AbKeR7nQdHPDddiDQ71YUsz1F138a7cJMfJVtpdYUSvE";
pub const PORT_FINANCE_RESERVE_SOL_COLLATERAL_MINT: &str =
    "Hk4Rp3kaPssB6hnjah3Mrqpt5CAXWGoqFT5dVsWA3TaM";

pub const LARIX_LENDING_MARKET: &str = "FRQHVH3U8vdTFHBaFZpsybzFAMofbnvnzgG1wFtrMVTG";
pub const LARIX_RESERVE_SOL: &str = "DfiaVGeHHtzvTGYqntUde8Pw6E8tgvMnHMnuM7CKXWss";
pub const LARIX_RESERVE_SOL_SUPPLY: &str = "976jcSPYeasM4ba4VkhGZna6S2o1DGN3WKvCSXYJXbRq";
pub const LARIX_RESERVE_SOL_COLLATERAL_MINT: &str = "23rfWYGvfCjVxJNW5Ce8E4xXXgjKgZTyJFwuQg6BMB4G";

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
