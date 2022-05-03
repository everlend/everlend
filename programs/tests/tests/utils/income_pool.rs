use super::{
    get_account, get_liquidity_mint, BanksClientResult, TestGeneralPool, TestIncomePoolMarket,
    TokenHolder, User,
};
use everlend_income_pools::{find_pool_program_address, instruction, state::IncomePool};
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[derive(Debug)]
pub struct TestIncomePool {
    pub pool_pubkey: Pubkey,
    pub token_mint_pubkey: Pubkey,
    pub token_account: Keypair,
}

impl TestIncomePool {
    pub fn new(
        test_income_pool_market: &TestIncomePoolMarket,
        token_mint_pubkey: Option<Pubkey>,
    ) -> Self {
        let token_mint_pubkey = token_mint_pubkey.unwrap_or(get_liquidity_mint().1);

        let (pool_pubkey, _) = find_pool_program_address(
            &everlend_income_pools::id(),
            &test_income_pool_market.keypair.pubkey(),
            &token_mint_pubkey,
        );

        Self {
            pool_pubkey,
            token_mint_pubkey,
            token_account: Keypair::new(),
        }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> IncomePool {
        let account = get_account(context, &self.pool_pubkey).await;
        IncomePool::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn create(
        &self,
        context: &mut ProgramTestContext,
        test_income_pool_market: &TestIncomePoolMarket,
    ) -> BanksClientResult<()> {
        let rent = context.banks_client.get_rent().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &context.payer.pubkey(),
                    &self.token_account.pubkey(),
                    rent.minimum_balance(spl_token::state::Account::LEN),
                    spl_token::state::Account::LEN as u64,
                    &spl_token::id(),
                ),
                instruction::create_pool(
                    &everlend_income_pools::id(),
                    &test_income_pool_market.keypair.pubkey(),
                    &self.token_mint_pubkey,
                    &self.token_account.pubkey(),
                    &test_income_pool_market.manager.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[
                &context.payer,
                &self.token_account,
                &test_income_pool_market.manager,
            ],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn deposit(
        &self,
        context: &mut ProgramTestContext,
        test_income_pool_market: &TestIncomePoolMarket,
        user: &TokenHolder,
        amount: u64,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::deposit(
                &everlend_income_pools::id(),
                &test_income_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &user.token_account,
                &self.token_account.pubkey(),
                &user.pubkey(),
                amount,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &user.owner],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn withdraw(
        &self,
        context: &mut ProgramTestContext,
        test_income_pool_market: &TestIncomePoolMarket,
        general_pool: &TestGeneralPool,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::withdraw(
                &everlend_income_pools::id(),
                &test_income_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &self.token_account.pubkey(),
                &general_pool.pool_pubkey,
                &general_pool.token_account.pubkey(),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
