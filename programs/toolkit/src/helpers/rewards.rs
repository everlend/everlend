use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use crate::Config;

pub fn init_rewards_root(
    config: &Config,
    reward_root_keypair: Keypair,
) -> Result<Pubkey, ClientError> {
    println!("Rewards root: {}", reward_root_keypair.pubkey());

    let tx = Transaction::new_with_payer(
        &[everlend_rewards::instruction::initialize_root(
            &everlend_rewards::id(),
            &reward_root_keypair.pubkey(),
            &config.fee_payer.pubkey()
        )],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), &reward_root_keypair]
    )?;

    Ok(reward_root_keypair.pubkey())
}