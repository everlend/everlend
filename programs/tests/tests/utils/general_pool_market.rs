use super::{get_account, BanksClientResult};
use everlend_general_pool::{instruction, state::PoolMarket};
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[derive(Debug)]
pub struct TestGeneralPoolMarket {
    pub keypair: Keypair,
    pub manager: Keypair,
}

impl TestGeneralPoolMarket {
    pub fn new() -> Self {
        Self {
            keypair: Keypair::new(),
            manager: Keypair::new(),
        }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> PoolMarket {
        let account = get_account(context, &self.keypair.pubkey()).await;
        PoolMarket::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn init(
        &self,
        context: &mut ProgramTestContext,
        registry: &Pubkey,
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
                    rent.minimum_balance(PoolMarket::LEN),
                    PoolMarket::LEN as u64,
                    &everlend_general_pool::id(),
                ),
                instruction::init_pool_market(
                    &everlend_general_pool::id(),
                    &self.keypair.pubkey(),
                    &self.manager.pubkey(),
                    registry,
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.keypair, &self.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
