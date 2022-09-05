use super::{get_account, BanksClientResult};
use everlend_registry::{
    instructions::{UpdateRegistryData, UpdateRegistryMarketsData},
    state::{Registry, RegistryMarkets},
};
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

    pub async fn get_registry_markets(&self, context: &mut ProgramTestContext) -> RegistryMarkets {
        let account = get_account(context, &self.keypair.pubkey()).await;
        RegistryMarkets::unpack_from_slice(&account.data).unwrap()
    }

    pub async fn init(&self, context: &mut ProgramTestContext) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::transfer(
                    &context.payer.pubkey(),
                    &self.manager.pubkey(),
                    999999999,
                ),
                everlend_registry::instruction::init(
                    &everlend_registry::id(),
                    &self.keypair.pubkey(),
                    &self.manager.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.keypair, &self.manager],
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
            Some(&self.manager.pubkey()),
            &[&self.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn update_registry_markets(
        &self,
        context: &mut ProgramTestContext,
        data: UpdateRegistryMarketsData,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[everlend_registry::instruction::update_registry_markets(
                &everlend_registry::id(),
                &self.keypair.pubkey(),
                &self.manager.pubkey(),
                data,
            )],
            Some(&self.manager.pubkey()),
            &[&self.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
