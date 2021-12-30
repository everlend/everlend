use super::{get_account, get_token_balance, TestPool, TestPoolMarket};
use everlend_ulp::{
    find_pool_borrow_authority_program_address, instruction,
    state::{Pool, PoolBorrowAuthority},
};
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_program_test::ProgramTestContext;
use solana_sdk::{signature::Signer, transaction::Transaction, transport};

pub const SHARE_ALLOWED: u16 = 10_000; // 100% of the total pool

#[derive(Debug)]
pub struct TestPoolBorrowAuthority {
    pub pool_borrow_authority_pubkey: Pubkey,
    pub borrow_authority: Pubkey,
}

impl TestPoolBorrowAuthority {
    pub fn new(test_pool: &TestPool, borrow_authority: Pubkey) -> Self {
        let (pool_borrow_authority_pubkey, _) = find_pool_borrow_authority_program_address(
            &everlend_ulp::id(),
            &test_pool.pool_pubkey,
            &borrow_authority,
        );

        Self {
            pool_borrow_authority_pubkey,
            borrow_authority,
        }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> PoolBorrowAuthority {
        let account = get_account(context, &self.pool_borrow_authority_pubkey).await;
        PoolBorrowAuthority::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn get_amount_allowed(&self, context: &mut ProgramTestContext) -> u64 {
        let pool_borrow_authority = self.get_data(context).await;
        let pool_account = get_account(context, &pool_borrow_authority.pool).await;
        let pool = Pool::unpack_unchecked(&pool_account.data).unwrap();
        let token_amount = get_token_balance(context, &pool.token_account).await;
        let total_amount_borrowed = pool.total_amount_borrowed;
        let total_pool_amount = token_amount + total_amount_borrowed;

        pool_borrow_authority
            .get_amount_allowed(total_pool_amount)
            .unwrap()
    }

    pub async fn create(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
        test_pool: &TestPool,
        share_allowed: u16,
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::create_pool_borrow_authority(
                &everlend_ulp::id(),
                &test_pool_market.keypair.pubkey(),
                &test_pool.pool_pubkey,
                &self.borrow_authority,
                &test_pool_market.manager.pubkey(),
                share_allowed,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &test_pool_market.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn update(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
        test_pool: &TestPool,
        share_allowed: u16,
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::update_pool_borrow_authority(
                &everlend_ulp::id(),
                &test_pool.pool_pubkey,
                &self.borrow_authority,
                &test_pool_market.manager.pubkey(),
                share_allowed,
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
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::delete_pool_borrow_authority(
                &everlend_ulp::id(),
                &test_pool.pool_pubkey,
                &self.borrow_authority,
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
