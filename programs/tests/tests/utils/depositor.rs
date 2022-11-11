use super::{
    get_account, get_liquidity_mint, BanksClientResult, TestGeneralPool, TestGeneralPoolMarket,
    TestIncomePool, TestIncomePoolMarket, TestLiquidityOracle, TestPool, TestPoolMarket,
    TestRegistry,
};
use everlend_depositor::{
    state::{Depositor, Rebalancing},
    RebalancingPDA,
};
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_utils::integrations::{self, MoneyMarketPubkeys};
use everlend_utils::PDA;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    signature::{Keypair, Signer},
    transaction::Transaction,
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
        let (rebalancing, _) = RebalancingPDA {
            depositor: self.depositor.pubkey(),
            mint: mint.clone(),
        }
        .find_address(&everlend_depositor::id());
        let account = get_account(context, &rebalancing).await;
        Rebalancing::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn init(
        &self,
        context: &mut ProgramTestContext,
        registry: &TestRegistry,
    ) -> BanksClientResult<()> {
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
                    &registry.keypair.pubkey(),
                    &self.depositor.pubkey(),
                    &context.payer.pubkey(),
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
        seed: Option<String>,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[everlend_depositor::instruction::create_transit(
                &everlend_depositor::id(),
                &self.depositor.pubkey(),
                token_mint,
                &context.payer.pubkey(),
                seed,
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
        registry: &TestRegistry,
        general_pool_market: &TestGeneralPoolMarket,
        general_pool: &TestGeneralPool,
        liquidity_oracle: &TestLiquidityOracle,
        refresh_income: bool,
        reserve_rates: DistributionArray,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[
                everlend_liquidity_oracle::instruction::update_reserve_rates(
                    &everlend_liquidity_oracle::id(),
                    &liquidity_oracle.keypair.pubkey(),
                    &context.payer.pubkey(),
                    &general_pool.token_mint_pubkey,
                    reserve_rates,
                ),
                everlend_depositor::instruction::start_rebalancing(
                    &everlend_depositor::id(),
                    &registry.keypair.pubkey(),
                    &self.depositor.pubkey(),
                    &general_pool.token_mint_pubkey,
                    &general_pool_market.keypair.pubkey(),
                    &general_pool.token_account.pubkey(),
                    &liquidity_oracle.keypair.pubkey(),
                    &context.payer.pubkey(),
                    refresh_income,
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn reset_rebalancing(
        &self,
        context: &mut ProgramTestContext,
        registry: &TestRegistry,
        liquidity_mint: &Pubkey,
        amount_to_distribute: u64,
        distributed_liquidity: DistributionArray,
        distribution_array: DistributionArray,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[everlend_depositor::instruction::reset_rebalancing(
                &everlend_depositor::id(),
                &registry.keypair.pubkey(),
                &self.depositor.pubkey(),
                liquidity_mint,
                &registry.manager.pubkey(),
                amount_to_distribute,
                distributed_liquidity,
                distribution_array,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &registry.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit(
        &self,
        context: &mut ProgramTestContext,
        registry: &TestRegistry,
        mm_pool_market: &TestPoolMarket,
        mm_pool: &TestPool,
        money_market_program_id: &Pubkey,
        money_market_pubkeys: &MoneyMarketPubkeys,
    ) -> BanksClientResult<()> {
        let liquidity_mint = get_liquidity_mint().1;
        let collateral_mint = mm_pool.token_mint_pubkey;

        let deposit_accounts =
            integrations::deposit_accounts(money_market_program_id, money_market_pubkeys);
        let deposit_collateral_storage_accounts = mm_pool.deposit_accounts(&mm_pool_market);

        let tx = Transaction::new_signed_with_payer(
            &[everlend_depositor::instruction::deposit(
                &everlend_depositor::id(),
                &registry.keypair.pubkey(),
                &self.depositor.pubkey(),
                &liquidity_mint,
                &collateral_mint,
                &context.payer.pubkey(),
                money_market_program_id,
                deposit_accounts,
                deposit_collateral_storage_accounts,
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
        registry: &TestRegistry,
        income_pool_market: &TestIncomePoolMarket,
        income_pool: &TestIncomePool,
        mm_pool_market: &TestPoolMarket,
        mm_pool: &TestPool,
        money_market_program_id: &Pubkey,
        money_market_pubkeys: &MoneyMarketPubkeys,
    ) -> BanksClientResult<()> {
        let collateral_mint = mm_pool.token_mint_pubkey;
        let liquidity_mint = get_liquidity_mint().1;

        let withdraw_accounts =
            integrations::withdraw_accounts(money_market_program_id, money_market_pubkeys);
        let collateral_storage_withdraw_accounts = mm_pool.withdraw_accounts(mm_pool_market, self);

        let tx = Transaction::new_signed_with_payer(
            &[everlend_depositor::instruction::withdraw(
                &everlend_depositor::id(),
                &registry.keypair.pubkey(),
                &self.depositor.pubkey(),
                &income_pool_market.keypair.pubkey(),
                &income_pool.token_account.pubkey(),
                &collateral_mint,
                &liquidity_mint,
                &context.payer.pubkey(),
                money_market_program_id,
                withdraw_accounts,
                collateral_storage_withdraw_accounts,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn refresh_mm_incomes(
        &self,
        context: &mut ProgramTestContext,
        registry: &TestRegistry,
        income_pool_market: &TestIncomePoolMarket,
        income_pool: &TestIncomePool,
        mm_pool_market: &TestPoolMarket,
        mm_pool: &TestPool,
        money_market_program_id: &Pubkey,
        money_market_pubkeys: &MoneyMarketPubkeys,
    ) -> BanksClientResult<()> {
        let collateral_mint = mm_pool.token_mint_pubkey;
        let liquidity_mint = get_liquidity_mint().1;

        let withdraw_accounts =
            integrations::withdraw_accounts(money_market_program_id, money_market_pubkeys);
        let collateral_storage_withdraw_accounts = mm_pool.withdraw_accounts(mm_pool_market, self);

        let bump_budget = ComputeBudgetInstruction::request_units(400_000u32, 0);

        let tx = Transaction::new_signed_with_payer(
            &[
                bump_budget,
                everlend_depositor::instruction::refresh_mm_incomes(
                    &everlend_depositor::id(),
                    &registry.keypair.pubkey(),
                    &self.depositor.pubkey(),
                    &income_pool_market.keypair.pubkey(),
                    &income_pool.token_account.pubkey(),
                    &collateral_mint,
                    &liquidity_mint,
                    &context.payer.pubkey(),
                    money_market_program_id,
                    withdraw_accounts,
                    collateral_storage_withdraw_accounts,
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
