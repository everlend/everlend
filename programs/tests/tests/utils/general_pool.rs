use super::BanksClientResult;
use super::{
    general_pool_borrow_authority::TestGeneralPoolBorrowAuthority, get_account, get_liquidity_mint,
    LiquidityProvider, TestGeneralPoolMarket, User,
};
use everlend_general_pool::find_pool_config_program_address;
use everlend_general_pool::state::{
    PoolConfig, SetPoolConfigParams, WithdrawalRequest, WithdrawalRequests,
};
use everlend_general_pool::{
    find_pool_program_address, find_transit_sol_unwrap_address,
    find_withdrawal_request_program_address, find_withdrawal_requests_program_address, instruction,
    state::Pool,
};
use everlend_utils::instructions::{config::initialize};
use solana_program::{
    instruction::AccountMeta, program_pack::Pack, pubkey::Pubkey, system_instruction,
    system_program, sysvar,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[derive(Debug)]
pub struct TestGeneralPool {
    pub pool_pubkey: Pubkey,
    pub pool_config_pubkey: Pubkey,
    pub token_mint_pubkey: Pubkey,
    pub token_account: Keypair,
    pub pool_mint: Keypair,
    pub config: Keypair,
    pub mining_reward_pool: Pubkey,
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

        let config = Keypair::new();

        let (mining_reward_pool, _) = Pubkey::find_program_address(
            &[
                b"reward_pool".as_ref(),
                &config.pubkey().to_bytes(),
                &token_mint_pubkey.to_bytes(),
            ],
            &everlend_rewards::id(),
        );

        let (pool_config_pubkey, _) =
            find_pool_config_program_address(&everlend_general_pool::id(), &pool_pubkey);

        Self {
            pool_pubkey,
            pool_config_pubkey,
            token_mint_pubkey,
            token_account: Keypair::new(),
            pool_mint: Keypair::new(),
            config,
            mining_reward_pool,
        }
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> Pool {
        let account = get_account(context, &self.pool_pubkey).await;
        Pool::unpack_unchecked(&account.data).unwrap()
    }

    pub async fn get_withdrawal_requests(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
    ) -> (Pubkey, WithdrawalRequests) {
        let (withdrawal_requests, _) = find_withdrawal_requests_program_address(
            &everlend_general_pool::id(),
            &test_pool_market.keypair.pubkey(),
            &self.token_mint_pubkey,
        );

        let account = get_account(context, &withdrawal_requests).await;
        (
            withdrawal_requests,
            WithdrawalRequests::unpack_unchecked(&account.data).unwrap(),
        )
    }

    pub async fn get_withdrawal_request(
        &self,
        context: &mut ProgramTestContext,
        withdrawal_requests: &Pubkey,
        from: &Pubkey,
    ) -> WithdrawalRequest {
        let (withdrawal_request, _) = find_withdrawal_request_program_address(
            &everlend_general_pool::id(),
            withdrawal_requests,
            from,
        );

        context
            .banks_client
            .get_account_data_with_borsh::<WithdrawalRequest>(withdrawal_request)
            .await
            .unwrap()
    }

    pub async fn create(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
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

        context.banks_client.process_transaction(tx).await.unwrap();

        // Initialize mining pool
        let tx = Transaction::new_signed_with_payer(
            &[
                initialize(
                    &eld_config::id(),
                    &self.config.pubkey(),
                    &context.payer.pubkey(),
                ),
                everlend_rewards::instruction::initialize_pool(
                    &everlend_rewards::id(),
                    &self.config.pubkey(),
                    &self.mining_reward_pool,
                    &self.token_mint_pubkey,
                    &self.pool_pubkey,
                    &context.payer.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.config],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn deposit(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
        user: &LiquidityProvider,
        mining_account: Pubkey,
        amount: u64,
    ) -> BanksClientResult<()> {
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
                &self.mining_reward_pool,
                &mining_account,
                &self.config.pubkey(),
                amount,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &user.owner],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn transfer_deposit(
        &self,
        context: &mut ProgramTestContext,
        user: &LiquidityProvider,
        destination_user: &LiquidityProvider,
        mining_account: Pubkey,
        destination_mining_account: Pubkey,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::transfer_deposit(
                &everlend_general_pool::id(),
                &self.pool_pubkey,
                &user.pool_account,
                &destination_user.pool_account,
                &user.owner.pubkey(),
                &destination_user.owner.pubkey(),
                &self.mining_reward_pool,
                &mining_account,
                &destination_mining_account,
                &self.config.pubkey(),
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
    ) -> BanksClientResult<()> {
        let mut addition_accounts: Vec<AccountMeta> = vec![];
        let mut destination = user.token_account;
        if self.token_mint_pubkey == spl_token::native_mint::id() {
            let (withdrawal_requests, _) = find_withdrawal_requests_program_address(
                &everlend_general_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.token_mint_pubkey,
            );
            let (withdrawal_request, _) = find_withdrawal_request_program_address(
                &everlend_general_pool::id(),
                &withdrawal_requests,
                &user.owner.pubkey(),
            );

            let (unwrap_sol_pubkey, _) =
                find_transit_sol_unwrap_address(&everlend_general_pool::id(), &withdrawal_request);

            addition_accounts = vec![
                AccountMeta::new_readonly(self.token_mint_pubkey, false),
                AccountMeta::new(unwrap_sol_pubkey, false),
                AccountMeta::new(context.payer.pubkey(), true),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ];

            destination = user.owner.pubkey();
        }

        let tx = Transaction::new_signed_with_payer(
            &[instruction::withdraw(
                &everlend_general_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &destination,
                &self.token_account.pubkey(),
                &self.token_mint_pubkey,
                &self.pool_mint.pubkey(),
                &user.owner.pubkey(),
                addition_accounts,
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
        mining_acc: Pubkey,
        collateral_amount: u64,
    ) -> BanksClientResult<()> {
        let mut destination = user.token_account;
        if self.token_mint_pubkey == spl_token::native_mint::id() {
            destination = user.owner.pubkey();
        };

        let tx = Transaction::new_signed_with_payer(
            &[instruction::withdraw_request(
                &everlend_general_pool::id(),
                &test_pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &user.pool_account,
                &destination,
                &self.token_account.pubkey(),
                &self.token_mint_pubkey,
                &self.pool_mint.pubkey(),
                &user.pubkey(),
                &self.mining_reward_pool,
                &mining_acc,
                &self.config.pubkey(),
                collateral_amount,
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
    ) -> BanksClientResult<()> {
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
    ) -> BanksClientResult<()> {
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
    ) -> BanksClientResult<()> {
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

    pub async fn init_user_mining(
        &self,
        context: &mut ProgramTestContext,
        _test_pool_market: &TestGeneralPoolMarket,
        user: &LiquidityProvider,
    ) -> Pubkey {
        let (mining_account, _) = Pubkey::find_program_address(
            &[
                b"mining".as_ref(),
                user.owner.pubkey().as_ref(),
                self.mining_reward_pool.as_ref(),
            ],
            &everlend_rewards::id(),
        );

        let tx = Transaction::new_signed_with_payer(
            &[everlend_rewards::instruction::initialize_mining(
                &everlend_rewards::id(),
                &self.config.pubkey(),
                &self.mining_reward_pool,
                &mining_account,
                &user.owner.pubkey(),
                &context.payer.pubkey(),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();
        mining_account
    }

    pub async fn migrate_withdraw_requests_account(
        &self,
        context: &mut ProgramTestContext,
        test_pool_market: &TestGeneralPoolMarket,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::migrate_instruction(
                &everlend_general_pool::id(),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &test_pool_market.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn set_pool_config(
        &self,
        context: &mut ProgramTestContext,
        pool_market: &TestGeneralPoolMarket,
        params: SetPoolConfigParams,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::set_pool_config(
                &everlend_general_pool::id(),
                &pool_market.keypair.pubkey(),
                &self.pool_pubkey,
                &pool_market.manager.pubkey(),
                params,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &pool_market.manager],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn get_pool_config(&self, context: &mut ProgramTestContext) -> PoolConfig {
        let account = get_account(context, &self.pool_config_pubkey).await;
        PoolConfig::unpack_unchecked(&account.data).unwrap()
    }
}
