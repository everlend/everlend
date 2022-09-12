use super::{get_account, BanksClientResult};

use everlend_liquidity_oracle::{
    find_token_distribution_program_address, instruction, state::DistributionArray,
    state::LiquidityOracle, state::TokenDistribution,
};
use solana_program_test::*;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{
    program_pack::Pack, signature::Keypair, signer::Signer, system_instruction,
    transaction::Transaction,
};

pub struct TestLiquidityOracle {
    pub keypair: Keypair,
}

impl TestLiquidityOracle {
    pub fn new() -> Self {
        TestLiquidityOracle {
            keypair: Keypair::new(),
        }
    }

    pub async fn init(&self, context: &mut ProgramTestContext) -> BanksClientResult<()> {
        let rent = context.banks_client.get_rent().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &context.payer.pubkey(),
                    &self.keypair.pubkey(),
                    rent.minimum_balance(LiquidityOracle::LEN),
                    LiquidityOracle::LEN as u64,
                    &everlend_liquidity_oracle::id(),
                ),
                instruction::init_liquidity_oracle(
                    &everlend_liquidity_oracle::id(),
                    &self.keypair.pubkey(),
                    &context.payer.pubkey(),
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer, &self.keypair],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn update(
        &self,
        context: &mut ProgramTestContext,
        authority: &Pubkey,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::update_liquidity_oracle_authority(
                &everlend_liquidity_oracle::id(),
                &self.keypair.pubkey(),
                &context.payer.pubkey(),
                authority,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn get_data(&self, context: &mut ProgramTestContext) -> LiquidityOracle {
        let account = get_account(context, &self.keypair.pubkey()).await;
        LiquidityOracle::unpack_unchecked(&account.data).unwrap()
    }
}

pub struct TestTokenDistribution {
    pub keypair: Keypair,
    pub token_mint: Pubkey,
    pub distribution: DistributionArray,
}

impl TestTokenDistribution {
    pub fn new(token_mint: Pubkey, distribution_array: DistributionArray) -> Self {
        TestTokenDistribution {
            keypair: Keypair::new(),
            token_mint,
            distribution: distribution_array,
        }
    }

    pub async fn init(
        &self,
        context: &mut ProgramTestContext,
        liquidity_oracle: &TestLiquidityOracle,
        authority: Pubkey,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::create_token_distribution(
                &everlend_liquidity_oracle::id(),
                &liquidity_oracle.keypair.pubkey(),
                &authority,
                &self.token_mint,
                self.distribution,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer, &context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn update(
        &self,
        context: &mut ProgramTestContext,
        liquidity_oracle: &TestLiquidityOracle,
        authority: Pubkey,
        distribution: DistributionArray,
    ) -> BanksClientResult<()> {
        let tx = Transaction::new_signed_with_payer(
            &[instruction::update_token_distribution(
                &everlend_liquidity_oracle::id(),
                &liquidity_oracle.keypair.pubkey(),
                &authority,
                &self.token_mint,
                distribution,
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );

        context.banks_client.process_transaction(tx).await
    }

    pub async fn get_data(
        &self,
        context: &mut ProgramTestContext,
        program_id: &Pubkey,
        liquidity_oracle: &TestLiquidityOracle,
    ) -> TokenDistribution {
        let (token_distribution, _) = find_token_distribution_program_address(
            program_id,
            &liquidity_oracle.keypair.pubkey(),
            &self.token_mint,
        );

        let account = get_account(context, &token_distribution).await;
        TokenDistribution::unpack_unchecked(&account.data).unwrap()
    }
}
