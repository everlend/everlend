use super::{
    get_account, get_reserve_account_data, pool_borrow_authority::TestPoolBorrowAuthority,
    TestLending, TestPool, TestPoolMarket,
};
use everlend_depositor::{find_transit_program_address, state::Depositor};
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

    #[allow(clippy::too_many_arguments)]
    pub async fn deposit(
        &self,
        context: &mut ProgramTestContext,
        spl_lending: &TestLending,
        general_pool_market: &TestPoolMarket,
        general_pool: &TestPool,
        general_pool_borrow_authority: &TestPoolBorrowAuthority,
        mm_pool_market: &TestPoolMarket,
        mm_pool: &TestPool,
        amount: u64,
    ) -> transport::Result<()> {
        // Rates should be refreshed
        context.warp_to_slot(10).unwrap();

        let (liquidity_transit_pubkey, _) = find_transit_program_address(
            &everlend_depositor::id(),
            &self.depositor.pubkey(),
            &general_pool.token_mint_pubkey,
        );

        let (collateral_transit_pubkey, _) = find_transit_program_address(
            &everlend_depositor::id(),
            &self.depositor.pubkey(),
            &mm_pool.token_mint_pubkey,
        );

        let reserve = get_reserve_account_data(context, &spl_lending.reserve_pubkey).await;

        let tx = Transaction::new_signed_with_payer(
            &[
                everlend_depositor::instruction::deposit(
                    &everlend_depositor::id(),
                    &self.depositor.pubkey(),
                    &general_pool_market.pool_market.pubkey(),
                    &general_pool.pool_pubkey,
                    &general_pool_borrow_authority.pool_borrow_authority_pubkey,
                    &general_pool.token_account.pubkey(),
                    &general_pool.token_mint_pubkey,
                    &self.rebalancer.pubkey(),
                    amount,
                ),
                spl_token_lending::instruction::deposit_reserve_liquidity(
                    spl_token_lending::id(),
                    amount,
                    liquidity_transit_pubkey,
                    collateral_transit_pubkey,
                    spl_lending.reserve_pubkey,
                    reserve.liquidity.supply_pubkey,
                    reserve.collateral.mint_pubkey,
                    spl_lending.market_pubkey,
                    self.rebalancer.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.rebalancer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
