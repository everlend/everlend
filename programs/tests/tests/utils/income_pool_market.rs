use super::{get_account, BanksClientResult, TestGeneralPoolMarket};
use everlend_income_pools::{instruction, state::IncomePoolMarket};
use solana_program::{program_pack::Pack, system_instruction};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[derive(Debug)]
pub struct TestIncomePoolMarket {
    pub keypair: Keypair,
    pub manager: Keypair,
}

impl TestIncomePoolMarket {
    pub fn new() -> Self {
        Self {
            keypair: Keypair::new(),
            manager: Keypair::new(),
        }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> IncomePoolMarket {
        let account = get_account(context, &self.keypair.pubkey()).await;
        IncomePoolMarket::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn init(
        &self,
        context: &mut ProgramTestContext,
        general_pool_market: &TestGeneralPoolMarket,
    ) -> BanksClientResult<()> {
        let rent = context.banks_client.get_rent().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
                // Transfer a few lamports to cover fee for create account
                system_instruction::transfer(
                    &context.payer.pubkey(),
                    &self.manager.pubkey(),
                    999999999,
                ),
                system_instruction::create_account(
                    &context.payer.pubkey(),
                    &self.keypair.pubkey(),
                    rent.minimum_balance(IncomePoolMarket::LEN),
                    IncomePoolMarket::LEN as u64,
                    &everlend_income_pools::id(),
                ),
                instruction::init_pool_market(
                    &everlend_income_pools::id(),
                    &self.keypair.pubkey(),
                    &self.manager.pubkey(),
                    &general_pool_market.keypair.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.keypair],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
