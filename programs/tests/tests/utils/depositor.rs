use super::{get_account, TestLiquidityOracle, TestPool, TestPoolMarket, TestSPLTokenLending};
use everlend_depositor::state::Depositor;
use everlend_utils::accounts;
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
}

impl TestDepositor {
    pub fn new() -> Self {
        let depositor = Keypair::new();
        Self { depositor }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> Depositor {
        let account = get_account(context, &self.depositor.pubkey()).await;
        Depositor::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn init(
        &self,
        context: &mut ProgramTestContext,
        general_pool_market: &TestPoolMarket,
        liquidity_oracle: &TestLiquidityOracle,
    ) -> transport::Result<()> {
        let rent = context.banks_client.get_rent().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
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
                    &general_pool_market.pool_market.pubkey(),
                    &liquidity_oracle.keypair.pubkey(),
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

    pub async fn start_rebalancing(
        &self,
        context: &mut ProgramTestContext,
        general_pool_market: &TestPoolMarket,
        general_pool: &TestPool,
        liquidity_oracle: &TestLiquidityOracle,
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[everlend_depositor::instruction::start_rebalancing(
                &everlend_depositor::id(),
                &self.depositor.pubkey(),
                &general_pool.token_mint_pubkey,
                &general_pool_market.pool_market.pubkey(),
                &general_pool.token_account.pubkey(),
                &liquidity_oracle.keypair.pubkey(),
                &context.payer.pubkey(),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit(
        &self,
        context: &mut ProgramTestContext,
        general_pool_market: &TestPoolMarket,
        general_pool: &TestPool,
        mm_pool_market: &TestPoolMarket,
        mm_pool: &TestPool,
        spl_token_lending: &TestSPLTokenLending,
        amount: u64,
    ) -> transport::Result<()> {
        let reserve = spl_token_lending.get_reserve_data(context).await;

        let liquidity_mint = general_pool.token_mint_pubkey;
        let collateral_mint = mm_pool.token_mint_pubkey;
        let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

        let money_market_accounts = accounts::spl_token_lending::deposit_or_redeem(
            &spl_token_lending.reserve_pubkey,
            &reserve.liquidity.supply_pubkey,
            &spl_token_lending.market_pubkey,
        );

        let tx = Transaction::new_signed_with_payer(
            &[everlend_depositor::instruction::deposit(
                &everlend_depositor::id(),
                &self.depositor.pubkey(),
                &general_pool_market.pool_market.pubkey(),
                &general_pool.token_account.pubkey(),
                &mm_pool_market.pool_market.pubkey(),
                &mm_pool.token_account.pubkey(),
                &mm_pool_collateral_mint,
                &liquidity_mint,
                &collateral_mint,
                &spl_token_lending::id(),
                money_market_accounts,
                amount,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn withdraw(
        &self,
        context: &mut ProgramTestContext,
        general_pool_market: &TestPoolMarket,
        general_pool: &TestPool,
        mm_pool_market: &TestPoolMarket,
        mm_pool: &TestPool,
        spl_token_lending: &TestSPLTokenLending,
        amount: u64,
    ) -> transport::Result<()> {
        let reserve = spl_token_lending.get_reserve_data(context).await;

        let collateral_mint = mm_pool.token_mint_pubkey;
        let liquidity_mint = general_pool.token_mint_pubkey;
        let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

        let money_market_accounts = accounts::spl_token_lending::deposit_or_redeem(
            &spl_token_lending.reserve_pubkey,
            &reserve.liquidity.supply_pubkey,
            &spl_token_lending.market_pubkey,
        );

        let tx = Transaction::new_signed_with_payer(
            &[everlend_depositor::instruction::withdraw(
                &everlend_depositor::id(),
                &self.depositor.pubkey(),
                &general_pool_market.pool_market.pubkey(),
                &general_pool.token_account.pubkey(),
                &mm_pool_market.pool_market.pubkey(),
                &mm_pool.token_account.pubkey(),
                &mm_pool_collateral_mint,
                &collateral_mint,
                &liquidity_mint,
                &spl_token_lending::id(),
                money_market_accounts,
                amount,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
