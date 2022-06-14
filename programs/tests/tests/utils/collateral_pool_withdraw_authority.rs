use super::{get_account, BanksClientResult, TestPool, TestPoolMarket};
use everlend_collateral_pool::{
    find_pool_withdraw_authority_program_address, instruction,
    state::{PoolWithdrawAuthority},
};
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_program_test::ProgramTestContext;
use solana_sdk::{signature::{Signer, Keypair}, transaction::Transaction};

#[derive(Debug)]
pub struct TestPoolWithdrawAuthority {
    pub pool_withdraw_authority_pubkey: Pubkey,
}

impl TestPoolWithdrawAuthority {
    pub fn new(test_pool: &TestPool, withdraw_authority: &Pubkey) -> Self {
        let (pool_withdraw_authority_pubkey, _) = find_pool_withdraw_authority_program_address(
            &everlend_collateral_pool::id(),
            &test_pool.pool_pubkey,
            &withdraw_authority,
        );

        Self {
            pool_withdraw_authority_pubkey,
        }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> PoolWithdrawAuthority {
        let account = get_account(context, &self.pool_withdraw_authority_pubkey).await;
        PoolWithdrawAuthority::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn create(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
        test_pool: &TestPool,
        withdraw_authority_pubkey: &Pubkey,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::create_pool_withdraw_authority(
                &everlend_collateral_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &test_pool.pool_pubkey,
                &withdraw_authority_pubkey,
                &test_pool_market.manager.pubkey(),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &test_pool_market.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn delete(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
        test_pool: &TestPool,
        withdraw_authority: Option<&Keypair>,
    ) -> BanksClientResult<()> {
        let withdraw_authority = withdraw_authority.unwrap_or(&context.payer);
        let tx = Transaction::new_signed_with_payer(
            &[instruction::delete_pool_borrow_authority(
                &everlend_collateral_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &test_pool.pool_pubkey,
                &withdraw_authority.pubkey(),
                &context.payer.pubkey(),
                &test_pool_market.manager.pubkey(),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &test_pool_market.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}

