use crate::utils::*;
use everlend_depositor::{instruction::InitMiningAccountsPubkeys, state::MiningType};
use everlend_utils::integrations::StakingMoneyMarket;
use port_finance_staking::state::stake_account;
use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use solana_program::{program_pack::Pack, system_instruction};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

const LARIX_MINING_SIZE: u64 = 1 + 32 + 32 + 1 + 16 + 560;

pub fn create_mining_account(
    config: &Config,
    staking_program_id: &Pubkey,
    mining_account: &Keypair,
    staking_money_market: StakingMoneyMarket,
) -> Result<(), ClientError> {
    let space = match staking_money_market {
        StakingMoneyMarket::Larix => LARIX_MINING_SIZE,
        StakingMoneyMarket::PortFinance => stake_account::StakeAccount::LEN as u64,
        // TODO return error
        _ => return Ok(()),
    };

    let rent = config
        .rpc_client
        .get_minimum_balance_for_rent_exemption(space as usize)?;
    let create_account_instruction = system_instruction::create_account(
        &config.fee_payer.pubkey(),
        &mining_account.pubkey(),
        rent,
        space,
        staking_program_id,
    );
    let tx = Transaction::new_with_payer(
        &[create_account_instruction],
        Some(&config.fee_payer.pubkey()),
    );

    config.sign_and_send_and_confirm_transaction(
        tx,
        vec![config.fee_payer.as_ref(), mining_account],
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn init_depositor_mining(
    config: &Config,
    pubkeys: InitMiningAccountsPubkeys,
    mining_type: MiningType,
) -> Result<(), ClientError> {
    let init_mining_instruction = everlend_depositor::instruction::init_mining_accounts(
        &everlend_depositor::id(),
        pubkeys,
        mining_type,
    );

    let tx =
        Transaction::new_with_payer(&[init_mining_instruction], Some(&config.fee_payer.pubkey()));

    config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

    Ok(())
}
