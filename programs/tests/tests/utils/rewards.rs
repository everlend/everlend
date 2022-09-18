use solana_program::pubkey::Pubkey;
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use everlend_utils::instructions::config::initialize;
use crate::utils::{BanksClientResult, get_liquidity_mint};

#[derive(Debug)]
pub struct TestRewards {
    pub token_mint_pubkey: Pubkey,
    pub root_account: Keypair,
    pub pool: Keypair,
    pub mining_reward_pool: Pubkey,
}

impl TestRewards {
    pub fn new(
        token_mint_pubkey: Option<Pubkey>,
    ) -> Self {
        let token_mint_pubkey = token_mint_pubkey.unwrap_or(get_liquidity_mint().1);

        let pool = Keypair::new();
        let root_account = Keypair::new();

        let (mining_reward_pool, _) = Pubkey::find_program_address(
            &[
                b"reward_pool".as_ref(),
                &root_account.pubkey().to_bytes(),
                &token_mint_pubkey.to_bytes(),
            ],
            &everlend_rewards::id(),
        );

        Self {
            pool,
            token_mint_pubkey,
            root_account,
            mining_reward_pool,
        }
    }

    pub async fn initialize_pool(
        &self,
        context: &mut ProgramTestContext,
    ) -> BanksClientResult<()> {
        // Initialize mining pool
        let tx = Transaction::new_signed_with_payer(
            &[
                initialize(
                    &eld_config::id(),
                    &self.root_account.pubkey(),
                    &context.payer.pubkey(),
                ),
                everlend_rewards::instruction::initialize_pool(
                    &everlend_rewards::id(),
                    &self.root_account.pubkey(),
                    &self.mining_reward_pool,
                    &self.token_mint_pubkey,
                    &self.pool.pubkey(),
                    &context.payer.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.root_account],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn initialize_mining(
        &self,
        context: &mut ProgramTestContext,
        user: &Pubkey,
    ) -> Pubkey {
        let (mining_account, _) = Pubkey::find_program_address(
            &[
                b"mining".as_ref(),
                user.as_ref(),
                self.mining_reward_pool.as_ref(),
            ],
            &everlend_rewards::id(),
        );

        let tx = Transaction::new_signed_with_payer(
            &[everlend_rewards::instruction::initialize_mining(
                &everlend_rewards::id(),
                &self.root_account.pubkey(),
                &self.mining_reward_pool,
                &mining_account,
                user,
                &context.payer.pubkey(),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        mining_account
    }

    pub async fn deposit_mining(
        &self,
        context: &mut ProgramTestContext,
        user: &Pubkey,
        mining_account: &Pubkey,
        amount: u64,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[everlend_rewards::instruction::deposit_mining(
                &everlend_rewards::id(),
                &self.root_account.pubkey(),
                &self.mining_reward_pool,
                &mining_account,
                user,
                &self.pool.pubkey(),
                amount,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.pool],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn withdraw_mining(
        &self,
        context: &mut ProgramTestContext,
        user: &Pubkey,
        mining_account: &Pubkey,
        amount: u64,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[everlend_rewards::instruction::withdraw_mining(
                &everlend_rewards::id(),
                &self.root_account.pubkey(),
                &self.mining_reward_pool,
                &mining_account,
                user,
                &self.pool.pubkey(),
                amount,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.pool],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn add_vault(
        &self,
        context: &mut ProgramTestContext,
        fee_account: &Pubkey,
    ) -> Pubkey {
        let (vault_pubkey, _) = Pubkey::find_program_address(
            &[b"vault".as_ref(), self.mining_reward_pool.as_ref(), self.token_mint_pubkey.as_ref()],
            &everlend_rewards::id(),
        );

        let tx = Transaction::new_signed_with_payer(
            &[everlend_rewards::instruction::add_vault(
                &everlend_rewards::id(),
                &self.root_account.pubkey(),
                &self.mining_reward_pool,
                &self.token_mint_pubkey,
                &vault_pubkey,
                fee_account,
                &context.payer.pubkey(),
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await.unwrap();

        vault_pubkey
    }

    pub async fn fill_vault(
        &self,
        context: &mut ProgramTestContext,
        fee_account: &Pubkey,
        from: &Pubkey,
        amount: u64,
    ) -> BanksClientResult<()> {
        let (vault_pubkey, _) = Pubkey::find_program_address(
            &[b"vault".as_ref(), self.mining_reward_pool.as_ref(), self.token_mint_pubkey.as_ref()],
            &everlend_rewards::id(),
        );

        let tx = Transaction::new_signed_with_payer(
            &[everlend_rewards::instruction::fill_vault(
                &everlend_rewards::id(),
                &self.root_account.pubkey(),
                &self.mining_reward_pool,
                &self.token_mint_pubkey,
                &vault_pubkey,
                fee_account,
                &context.payer.pubkey(),
                from,
                amount
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn claim(
        &self,
        context: &mut ProgramTestContext,
        user: &Keypair,
        mining_account: &Pubkey,
        user_reward_token: &Pubkey
    ) -> BanksClientResult<()> {
        let (vault_pubkey, _) = Pubkey::find_program_address(
            &[b"vault".as_ref(), self.mining_reward_pool.as_ref(), self.token_mint_pubkey.as_ref()],
            &everlend_rewards::id(),
        );

        let tx = Transaction::new_signed_with_payer(
            &[everlend_rewards::instruction::claim(
                &everlend_rewards::id(),
                &self.root_account.pubkey(),
                &self.mining_reward_pool,
                &self.token_mint_pubkey,
                &vault_pubkey,
                mining_account,
                &user.pubkey(),
                user_reward_token,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, user],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }
}
