use super::{
    get_account, TestIncomePool, TestIncomePoolMarket, TestLiquidityOracle, TestPool,
    TestPoolMarket, TestSPLTokenLending,
};
use everlend_depositor::{
    find_rebalancing_program_address,
    state::{Depositor, Rebalancing},
};
use everlend_utils::integrations::{self, MoneyMarketPubkeys};
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

    pub async fn get_rebalancing_data(
        &self,
        context: &mut ProgramTestContext,
        mint: &Pubkey,
    ) -> Rebalancing {
        let (rebalancing, _) = find_rebalancing_program_address(
            &everlend_depositor::id(),
            &self.depositor.pubkey(),
            mint,
        );
        let account = get_account(context, &rebalancing).await;
        Rebalancing::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn init(
        &self,
        context: &mut ProgramTestContext,
        general_pool_market: &TestPoolMarket,
        income_pool_market: &TestIncomePoolMarket,
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
                    &general_pool_market.keypair.pubkey(),
                    &income_pool_market.keypair.pubkey(),
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
                &general_pool_market.keypair.pubkey(),
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
        test_spl_token_lending: &TestSPLTokenLending,
    ) -> transport::Result<()> {
        let reserve = test_spl_token_lending.get_reserve_data(context).await;

        let liquidity_mint = general_pool.token_mint_pubkey;
        let collateral_mint = mm_pool.token_mint_pubkey;
        let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

        let money_market_pubkeys = integrations::spl_token_lending::AccountPubkeys {
            reserve: test_spl_token_lending.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: test_spl_token_lending.market_pubkey,
        };

        let deposit_accounts = integrations::deposit_accounts(
            &spl_token_lending::id(),
            &MoneyMarketPubkeys::SPL(money_market_pubkeys),
        );

        let tx = Transaction::new_signed_with_payer(
            &[everlend_depositor::instruction::deposit(
                &everlend_depositor::id(),
                &self.depositor.pubkey(),
                &general_pool_market.keypair.pubkey(),
                &general_pool.token_account.pubkey(),
                &mm_pool_market.keypair.pubkey(),
                &mm_pool.token_account.pubkey(),
                &mm_pool_collateral_mint,
                &liquidity_mint,
                &collateral_mint,
                &spl_token_lending::id(),
                deposit_accounts,
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
        income_pool_market: &TestIncomePoolMarket,
        income_pool: &TestIncomePool,
        mm_pool_market: &TestPoolMarket,
        mm_pool: &TestPool,
        test_spl_token_lending: &TestSPLTokenLending,
    ) -> transport::Result<()> {
        let reserve = test_spl_token_lending.get_reserve_data(context).await;

        let collateral_mint = mm_pool.token_mint_pubkey;
        let liquidity_mint = general_pool.token_mint_pubkey;
        let mm_pool_collateral_mint = mm_pool.pool_mint.pubkey();

        let money_market_pubkeys = integrations::spl_token_lending::AccountPubkeys {
            reserve: test_spl_token_lending.reserve_pubkey,
            reserve_liquidity_supply: reserve.liquidity.supply_pubkey,
            reserve_liquidity_oracle: reserve.liquidity.oracle_pubkey,
            lending_market: test_spl_token_lending.market_pubkey,
        };

        let withdraw_accounts = integrations::withdraw_accounts(
            &spl_token_lending::id(),
            &MoneyMarketPubkeys::SPL(money_market_pubkeys),
        );

        let tx = Transaction::new_signed_with_payer(
            &[everlend_depositor::instruction::withdraw(
                &everlend_depositor::id(),
                &self.depositor.pubkey(),
                &general_pool_market.keypair.pubkey(),
                &general_pool.token_account.pubkey(),
                &income_pool_market.keypair.pubkey(),
                &income_pool.token_account.pubkey(),
                &mm_pool_market.keypair.pubkey(),
                &mm_pool.token_account.pubkey(),
                &mm_pool_collateral_mint,
                &collateral_mint,
                &liquidity_mint,
                &spl_token_lending::id(),
                withdraw_accounts,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
