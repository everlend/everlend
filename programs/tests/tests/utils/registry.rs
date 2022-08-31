use super::{get_account, BanksClientResult};
use everlend_registry::{instructions::UpdateRegistryData, state::Registry};
use solana_program::{program_pack::Pack, system_instruction};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[derive(Debug)]
pub struct TestRegistry {
    pub keypair: Keypair,
    pub manager: Keypair,
}

impl TestRegistry {
    pub fn new() -> Self {
        Self {
            keypair: Keypair::new(),
            manager: Keypair::new(),
        }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> Registry {
        let account = get_account(context, &self.keypair.pubkey()).await;
        Registry::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn init(&self, context: &mut ProgramTestContext) -> BanksClientResult<()> {
        let rent = context.banks_client.get_rent().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::transfer(
                    &context.payer.pubkey(),
                    &self.manager.pubkey(),
                    999999999,
                ),
                system_instruction::create_account(
                    &context.payer.pubkey(),
                    &self.keypair.pubkey(),
                    rent.minimum_balance(Registry::LEN),
                    Registry::LEN as u64,
                    &everlend_registry::id(),
                ),
                everlend_registry::instruction::init(
                    &everlend_registry::id(),
                    &self.keypair.pubkey(),
                    &self.manager.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.keypair],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn update_registry(
        &self,
        context: &mut ProgramTestContext,
        data: UpdateRegistryData,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[everlend_registry::instruction::update_registry(
                &everlend_registry::id(),
                &self.keypair.pubkey(),
                &self.manager.pubkey(),
                data,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
