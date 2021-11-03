use super::{pool_borrow_authority::TestPoolBorrowAuthority, TestPool, TestPoolMarket};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport,
};

#[derive(Debug)]
pub struct TestRebalancing {
    pub rebalancer: Keypair,
}

impl TestRebalancing {
    pub fn new(rebalancer: Option<Keypair>) -> Self {
        let rebalancer = rebalancer.unwrap_or_else(Keypair::new);
        Self { rebalancer }
    }

    pub async fn deposit(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
        test_pool: &TestPool,
        test_pool_borrow_authority: &TestPoolBorrowAuthority,
        amount: u64,
    ) -> transport::Result<()> {
        let rent = context.banks_client.get_rent().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
                everlend_depositor::instruction::deposit(
                    &everlend_depositor::id(),
                    &test_pool_market.pool_market.pubkey(),
                    &test_pool.pool_pubkey,
                    &test_pool_borrow_authority.pool_borrow_authority_pubkey,
                    &test_pool.token_account.pubkey(),
                    &staging_token_account_pubkey,
                    &self.rebalancer.pubkey(),
                    amount,
                ),
                // spl_token_lending::instruction::deposit_reserve_liquidity(
                //     &spl_token_lending::id(),
                //     amount,
                //     source_liquidity_pubkey,
                //     destination_collateral_pubkey,
                //     reserve_pubkey,
                //     reserve_liquidity_supply_pubkey,
                //     reserve_collateral_mint_pubkey,
                //     lending_market_pubkey,
                //     user_transfer_authority_pubkey,
                // ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.rebalancer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
