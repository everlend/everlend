use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{Keypair, Signature},
    signer::Signer,
    signers::Signers,
    transaction::Transaction,
};

pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

pub const SOL_ORACLE: &str = "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix";

pub const PORT_FINANCE_LENDING_MARKET: &str = "H27Quk3DSbu55T4dCr1NddTTSAezXwHU67FPCZVKLhSW";
pub const PORT_FINANCE_RESERVE_SOL: &str = "6FeVStQAGPWvfWijDHF7cTWRCi7He6vTT3ubfNhe9SPt";
pub const PORT_FINANCE_RESERVE_SOL_SUPPLY: &str = "AbKeR7nQdHPDddiDQ71YUsz1F138a7cJMfJVtpdYUSvE";
pub const PORT_FINANCE_RESERVE_SOL_COLLATERAL: &str =
    "Hk4Rp3kaPssB6hnjah3Mrqpt5CAXWGoqFT5dVsWA3TaM";

pub struct Config {
    pub rpc_client: RpcClient,
    pub verbose: bool,
    pub fee_payer: Keypair,
}

pub fn sign_and_send_and_confirm_transaction<T: Signers>(
    config: &Config,
    mut tx: Transaction,
    keypairs: &T,
) -> Result<Signature, ClientError> {
    let (recent_blockhash, _) = config.rpc_client.get_recent_blockhash()?;

    tx.try_sign(keypairs, recent_blockhash)?;

    let signature = config
        .rpc_client
        .send_and_confirm_transaction_with_spinner(&tx)?;
    // println!("Signature: {}", signature);

    Ok(signature)
}

// pub fn transfer(
//     config: &Config,
//     from: &Keypair,
//     to: &Pubkey,
//     amount: u64,
// ) -> Result<(), ClientError> {
//     let mut tx = Transaction::new_with_payer(
//         &[system_instruction::transfer(&from.pubkey(), to, amount)],
//         Some(&from.pubkey()),
//     );

//     let (recent_blockhash, _) = config.rpc_client.get_recent_blockhash()?;

//     tx.try_sign(&[from], recent_blockhash)?;

//     let signature = config.rpc_client.send_transaction(&tx).unwrap();
//     println!("Signature: {}", signature);

//     Ok(())
// }

// pub fn spl_transfer(
//     config: &Config,
//     source: &Pubkey,
//     destination: &Pubkey,
//     authority: &Keypair,
//     amount: u64,
// ) -> Result<(), ClientError> {
//     let mut tx = Transaction::new_with_payer(
//         &[spl_token::instruction::transfer(
//             &spl_token::id(),
//             source,
//             destination,
//             &authority.pubkey(),
//             &[],
//             amount,
//         )
//         .unwrap()],
//         Some(&config.fee_payer.pubkey()),
//     );

//     let (recent_blockhash, _) = config.rpc_client.get_recent_blockhash()?;

//     tx.try_sign(&[authority], recent_blockhash)?;

//     let signature = config.rpc_client.send_transaction(&tx).unwrap();
//     println!("Signature: {}", signature);

//     Ok(())
// }

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

    sign_and_send_and_confirm_transaction(config, tx, &[&config.fee_payer])?;

    let associated_token_address =
        spl_associated_token_account::get_associated_token_address(wallet, mint);

    Ok(associated_token_address)
}

// pub fn spl_create_token_account(
//     config: &Config,
//     account: Option<Keypair>,
//     mint: &Pubkey,
//     manager: &Pubkey,
// ) -> Result<Pubkey, ClientError> {
//     let account = account.unwrap_or_else(Keypair::new);

//     println!("Creating token account {}", account.pubkey());

//     let market_balance = config
//         .rpc_client
//         .get_minimum_balance_for_rent_exemption(spl_token::state::Account::LEN)?;

//     let mut tx = Transaction::new_with_payer(
//         &[
//             system_instruction::create_account(
//                 &config.fee_payer.pubkey(),
//                 &account.pubkey(),
//                 market_balance,
//                 spl_token::state::Account::LEN as u64,
//                 &spl_token::id(),
//             ),
//             spl_token::instruction::initialize_account(
//                 &spl_token::id(),
//                 &account.pubkey(),
//                 mint,
//                 manager,
//             )
//             .unwrap(),
//         ],
//         Some(&config.fee_payer.pubkey()),
//     );

//     let (recent_blockhash, _) = config.rpc_client.get_recent_blockhash()?;

//     tx.try_sign(&[&config.fee_payer, &account], recent_blockhash)?;

//     let signature = config.rpc_client.send_transaction(&tx).unwrap();
//     println!("Signature: {}", signature);

//     Ok(account.pubkey())
// }
