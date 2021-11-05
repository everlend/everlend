use super::{
    get_account, pool_borrow_authority::TestPoolBorrowAuthority, TestPool, TestPoolMarket,
};
use everlend_depositor::state::Depositor;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport,
};

#[derive(Debug)]
pub struct TestDepositor {
    pub depositor: Keypair,
    pub rebalancer: Keypair,
}

impl TestDepositor {
    pub fn new(rebalancer: Option<Keypair>) -> Self {
        let depositor = Keypair::new();
        let rebalancer = rebalancer.unwrap_or_else(Keypair::new);
        Self {
            depositor,
            rebalancer,
        }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> Depositor {
        let account = get_account(context, &self.depositor.pubkey()).await;
        Depositor::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn init(&self, context: &mut ProgramTestContext) -> transport::Result<()> {
        let rent = context.banks_client.get_rent().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
                // Transfer some lamports to cover fee for rebalancer
                system_instruction::transfer(
                    &context.payer.pubkey(),
                    &self.rebalancer.pubkey(),
                    999999999,
                ),
                system_instruction::create_account(
                    &context.payer.pubkey(),
                    &self.depositor.pubkey(),
                    rent.minimum_balance(Depositor::LEN),
                    Depositor::LEN as u64,
                    &everlend_depositor::id(),
                ),
                everlend_depositor::instruction::init(
                    &everlend_depositor::id(),
                    &self.depositor.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.depositor],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn create_transit(
        &self,
        context: &mut ProgramTestContext,
        token_mint: &Pubkey,
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[everlend_depositor::instruction::create_transit(
                &everlend_depositor::id(),
                &self.depositor.pubkey(),
                token_mint,
                &context.payer.pubkey(),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn deposit(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
        test_pool: &TestPool,
        test_pool_borrow_authority: &TestPoolBorrowAuthority,
        amount: u64,
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[
                everlend_depositor::instruction::deposit(
                    &everlend_depositor::id(),
                    &self.depositor.pubkey(),
                    &test_pool_market.pool_market.pubkey(),
                    &test_pool.pool_pubkey,
                    &test_pool_borrow_authority.pool_borrow_authority_pubkey,
                    &test_pool.token_account.pubkey(),
                    &test_pool.token_mint.pubkey(),
                    &self.rebalancer.pubkey(),
                    amount,
                ),
                // spl_token_lending::instruction::deposit_reserve_liquidity(
                //     spl_token_lending::id(),
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
