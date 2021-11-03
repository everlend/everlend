use super::{create_token_account, mint_tokens, TestPool};
use solana_program::pubkey::Pubkey;
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    transport,
};

pub trait User {
    fn pubkey(&self) -> Pubkey;
}

#[derive(Debug)]
pub struct LiquidityProvider {
    pub owner: Keypair,
    pub token_account: Pubkey,
    pub pool_account: Pubkey,
}

impl User for LiquidityProvider {
    fn pubkey(&self) -> Pubkey {
        self.owner.pubkey()
    }
}

pub async fn add_liquidity_provider(
    context: &mut ProgramTestContext,
    test_pool: &TestPool,
    mint_amount: u64,
) -> transport::Result<LiquidityProvider> {
    let user = Keypair::new();
    let token_account = Keypair::new();
    let pool_account = Keypair::new();

    create_token_account(
        context,
        &token_account,
        &test_pool.token_mint.pubkey(),
        &user.pubkey(),
    )
    .await?;

    create_token_account(
        context,
        &pool_account,
        &test_pool.pool_mint.pubkey(),
        &user.pubkey(),
    )
    .await?;

    mint_tokens(
        context,
        &test_pool.token_mint.pubkey(),
        &token_account.pubkey(),
        mint_amount,
    )
    .await?;

    Ok(LiquidityProvider {
        owner: user,
        token_account: token_account.pubkey(),
        pool_account: pool_account.pubkey(),
    })
}

// pub struct Rebalancer {
//     pub owner: Keypair,
//     pub token_account: Pubkey,
// }

// impl User for Rebalancer {
//     fn pubkey(&self) -> Pubkey {
//         self.owner.pubkey()
//     }
// }

// pub async fn add_rebalancer(
//     context: &mut ProgramTestContext,
//     test_pool: &TestPool,
// ) -> transport::Result<Rebalancer> {
//     let user = Keypair::new();
//     let token_account = Keypair::new();

//     create_token_account(
//         context,
//         &token_account,
//         &test_pool.token_mint.pubkey(),
//         &user.pubkey(),
//     )
//     .await?;

//     Ok(Rebalancer {
//         owner: user,
//         token_account: token_account.pubkey(),
//     })
// }
