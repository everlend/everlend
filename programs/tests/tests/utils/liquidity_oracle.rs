use super::get_account;

use everlend_lo::{id, instruction, state::LiquidityOracle};
use solana_program_test::*;
use solana_sdk::{
    program_pack::Pack, signature::Keypair, signer::Signer, system_instruction,
    transaction::Transaction, transport,
};

pub struct TestLiquidityOracle {
    pub keypair: Keypair,
}

impl TestLiquidityOracle {
    pub fn new() -> Self {
        TestLiquidityOracle {
            keypair: Keypair::new(),
        }
    }

    pub async fn init(&self, context: &mut ProgramTestContext) -> transport::Result<()> {
        let rent = context.banks_client.get_rent().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &context.payer.pubkey(),
                    &self.keypair.pubkey(),
                    rent.minimum_balance(LiquidityOracle::LEN),
                    LiquidityOracle::LEN as u64,
                    &id(),
                ),
                instruction::init_liquidity_oracle(
                    &id(),
                    &self.keypair.pubkey(),
                    &context.payer.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.keypair],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn update(
        &self,
        context: &mut ProgramTestContext,
        value: [u8; 32],
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::update_liquidity_oracle_authority(
                &id(),
                &self.keypair.pubkey(),
                &context.payer.pubkey(),
                value,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> LiquidityOracle {
        let account = get_account(context, &self.keypair.pubkey()).await;
        LiquidityOracle::unpack_unchecked(&account.data).unwrap()
    }
}
