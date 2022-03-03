use super::{
    general_pool_borrow_authority::TestGeneralPoolBorrowAuthority, get_account, get_liquidity_mint,
    LiquidityProvider, TestGeneralPoolMarket, User,
};
use everlend_general_pool::state::{WithdrawalRequest, WithdrawalRequests};
use everlend_general_pool::{
    find_pool_program_address, find_user_withdrawal_request_program_address, find_withdrawal_requests_program_address, instruction, state::Pool,
};
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport,
};

#[derive(Debug)]
pub struct TestGeneralPool {
    pub pool_pubkey: Pubkey,
    pub token_mint_pubkey: Pubkey,
    pub token_account: Keypair,
    pub pool_mint: Keypair,
}

impl TestGeneralPool {
    pub fn new(
        test_pool_market: &TestGeneralPoolMarket,
        token_mint_pubkey: Option<Pubkey>,
    ) -> Self {
        let token_mint_pubkey = token_mint_pubkey.unwrap_or(get_liquidity_mint().1);

        let (pool_pubkey, _) = find_pool_program_address(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &token_mint_pubkey,
        );

        Self {
            pool_pubkey,
            token_mint_pubkey,
            token_account: Keypair::new(),
            pool_mint: Keypair::new(),
        }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> Pool {
        let account = get_account(context, &self.pool_pubkey).await;
        Pool::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn get_withdraw_requests(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
        program_id: &Pubkey,
    ) -> WithdrawalRequests {
        let (withdrawal_requests, _) = find_withdrawal_requests_program_address(
            program_id,
            &test_pool_market.keypair.pubkey(),
            &self.token_mint_pubkey,
        );

        let account = get_account(context, &withdrawal_requests).await;
        WithdrawalRequests::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn get_user_withdraw_requests(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
        index: u64,
        program_id: &Pubkey,
    ) -> WithdrawalRequest {
        let (user_withdraw_request, _) = find_user_withdrawal_request_program_address(
            program_id,
            &test_pool_market.keypair.pubkey(),
            &self.token_mint_pubkey,
            index,
        );

        let account = get_account(context, &user_withdraw_request).await;
        WithdrawalRequest::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn create(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
    ) -> transport::Result<()> {
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
                system_instruction::create_account(
                    &context.payer.pubkey(),
                    &self.pool_mint.pubkey(),
                    rent.minimum_balance(spl_token::state::Mint::LEN),
                    spl_token::state::Mint::LEN as u64,
                    &spl_token::id(),
                ),
                instruction::create_pool(
                    &everlend_general_pool::id(),
                    &test_pool_market.keypair.pubkey(),
                    &self.token_mint_pubkey,
                    &self.token_account.pubkey(),
                    &self.pool_mint.pubkey(),
                    &test_pool_market.manager.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[
                &context.payer,
                &self.token_account,
                &self.pool_mint,
                &test_pool_market.manager,
            ],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn deposit(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
        user: &LiquidityProvider,
        amount: u64,
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::deposit(
                &everlend_general_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &user.token_account,
                &user.pool_account,
                &self.token_account.pubkey(),
                &self.pool_mint.pubkey(),
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
        test_pool_market: &TestGeneralPoolMarket,
        user: &LiquidityProvider,
        index: u64,
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::withdraw(
                &everlend_general_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &user.token_account,
                &self.token_account.pubkey(),
                &self.token_mint_pubkey,
                &self.pool_mint.pubkey(),
                &user.owner.pubkey(),
                index,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn withdraw_request(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
        user: &LiquidityProvider,
        amount: u64,
        index: u64,
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::withdraw_request(
                &everlend_general_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &user.pool_account,
                &user.token_account,
                &self.token_account.pubkey(),
                &self.token_mint_pubkey,
                &self.pool_mint.pubkey(),
                &user.pubkey(),
                amount,
                index
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &user.owner],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn cancel_withdraw_request(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
        user: &LiquidityProvider,
        index: u64,
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::cancel_withdraw_request(
                &everlend_general_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &user.pool_account,
                &self.token_mint_pubkey,
                &self.pool_mint.pubkey(),
                &test_pool_market.manager.pubkey(),
                &user.owner.pubkey(),
                index,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &test_pool_market.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn borrow(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
        test_pool_borrow_authority: &TestGeneralPoolBorrowAuthority,
        borrow_authority: Option<&Keypair>,
        destination: &Pubkey,
        amount: u64,
    ) -> transport::Result<()> {
        let borrow_authority = borrow_authority.unwrap_or(&context.payer);

        let tx = Transaction::new_signed_with_payer(
            &[instruction::borrow(
                &everlend_general_pool::id(),
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
        test_pool_market: &TestGeneralPoolMarket,
        test_pool_borrow_authority: &TestGeneralPoolBorrowAuthority,
        user: &LiquidityProvider,
        amount: u64,
        interest_amount: u64,
    ) -> transport::Result<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::repay(
                &everlend_general_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &test_pool_borrow_authority.pool_borrow_authority_pubkey,
                &user.token_account,
                &self.token_account.pubkey(),
                &user.pubkey(),
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