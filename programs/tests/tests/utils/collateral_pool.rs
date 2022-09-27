use super::{
    get_account, get_liquidity_mint, collateral_pool_borrow_authority::TestPoolBorrowAuthority,
    BanksClientResult, TestPoolMarket, TestPoolWithdrawAuthority,
};
use everlend_utils::find_program_address;
use everlend_collateral_pool::{find_pool_program_address, find_pool_withdraw_authority_program_address, instruction, state::Pool};
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program::instruction::AccountMeta;
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use crate::utils::TestDepositor;
use super::collateral_pool_liquidity_provider::{LiquidityProvider};

#[derive(Debug)]
pub struct TestPool {
    pub pool_pubkey: Pubkey,
    pub token_mint_pubkey: Pubkey,
    pub token_account: Keypair,
}

impl TestPool {
    pub fn new(test_pool_market: &TestPoolMarket, token_mint_pubkey: Option<Pubkey>) -> Self {
        let token_mint_pubkey = token_mint_pubkey.unwrap_or(get_liquidity_mint().1);

        let (pool_pubkey, _) = find_pool_program_address(
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &token_mint_pubkey,
        );

        Self {
            pool_pubkey,
            token_mint_pubkey,
            token_account: Keypair::new(),
        }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> Pool {
        let account = get_account(context, &self.pool_pubkey).await;
        Pool::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn create(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
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
                    &everlend_collateral_pool::id(),
                    &test_pool_market.keypair.pubkey(),
                    &self.token_mint_pubkey,
                    &self.token_account.pubkey(),
                    &test_pool_market.manager.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[
                &context.payer,
                &self.token_account,
                &test_pool_market.manager,
            ],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub fn deposit_accounts(
        &self,
        test_pool_market: &TestPoolMarket) -> Vec<AccountMeta> {

        let (collateral_pool_market_authority, _) =
            find_program_address(&everlend_collateral_pool::id(), &test_pool_market.keypair.pubkey());
        let (collateral_pool, _) = everlend_collateral_pool::find_pool_program_address(
            &everlend_collateral_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &self.token_mint_pubkey,
        );

        vec![
            AccountMeta::new_readonly(test_pool_market.keypair.pubkey(), false),
            AccountMeta::new_readonly(collateral_pool_market_authority, false),
            AccountMeta::new_readonly(collateral_pool, false),
            AccountMeta::new(self.token_account.pubkey(), false),
            AccountMeta::new_readonly(everlend_collateral_pool::id(), false),
        ]
    }

    pub async fn deposit(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
        user: &LiquidityProvider,
        amount: u64,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::deposit(
                &everlend_collateral_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &user.token_account,
                &self.token_account.pubkey(),
                &user.owner.pubkey(),
                amount,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &user.owner],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub fn withdraw_accounts(
        &self,
        test_pool_market: &TestPoolMarket,
        depositor: &TestDepositor,
    ) -> Vec<AccountMeta> {

        let (collateral_pool_market_authority, _) =
            find_program_address(&everlend_collateral_pool::id(), &test_pool_market.keypair.pubkey());
        let (collateral_pool, _) = everlend_collateral_pool::find_pool_program_address(
         &everlend_collateral_pool::id(),
         &test_pool_market.keypair.pubkey(),
         &self.token_mint_pubkey,
         );

        let (depositor_authority, _) = find_program_address(&everlend_depositor::id(), &depositor.depositor.pubkey());

        let (collateral_pool_withdraw_authority, _) = find_pool_withdraw_authority_program_address(
            &everlend_collateral_pool::id(),
            &collateral_pool,
            &depositor_authority,
        );

            vec![
                AccountMeta::new_readonly(test_pool_market.keypair.pubkey(), false),
                AccountMeta::new_readonly(collateral_pool_market_authority, false),
                AccountMeta::new_readonly(collateral_pool, false),
                AccountMeta::new(self.token_account.pubkey(), false),
                AccountMeta::new_readonly(everlend_collateral_pool::id(), false),
                AccountMeta::new_readonly(collateral_pool_withdraw_authority, false),
            ]
    }

    pub async fn withdraw(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
        pool_withdraw_authority: &TestPoolWithdrawAuthority,
        withdraw_authority: Option<&Keypair>,
        user: &LiquidityProvider,
        amount: u64,
    ) -> BanksClientResult<()> {
        let withdraw_authority = withdraw_authority.unwrap_or(&context.payer);
        let tx = Transaction::new_signed_with_payer(
            &[instruction::withdraw(
                &everlend_collateral_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &pool_withdraw_authority.pool_withdraw_authority_pubkey,
                &user.token_account,
                &self.token_account.pubkey(),
                &withdraw_authority.pubkey(),
                amount,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, withdraw_authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn borrow(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
        test_pool_borrow_authority: &TestPoolBorrowAuthority,
        borrow_authority: Option<&Keypair>,
        destination: &Pubkey,
        amount: u64,
    ) -> BanksClientResult<()> {
        let borrow_authority = borrow_authority.unwrap_or(&context.payer);

        let tx = Transaction::new_signed_with_payer(
            &[instruction::borrow(
                &everlend_collateral_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &test_pool_borrow_authority.pool_borrow_authority_pubkey,
                destination,
                &self.token_account.pubkey(),
                &borrow_authority.pubkey(),
                amount,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, borrow_authority],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn repay(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestPoolMarket,
        test_pool_borrow_authority: &TestPoolBorrowAuthority,
        user: &LiquidityProvider,
        amount: u64,
        interest_amount: u64,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::repay(
                &everlend_collateral_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &test_pool_borrow_authority.pool_borrow_authority_pubkey,
                &user.token_account,
                &self.token_account.pubkey(),
                &user.owner.pubkey(),
                amount,
                interest_amount,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &user.owner],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}

