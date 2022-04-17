use super::{create_token_account, mint_tokens, transfer, BanksClientResult, SOL_MINT};
use solana_program::pubkey::Pubkey;
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};
use std::str::FromStr;

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
    token_mint_pubkey: &Pubkey,
    pool_mint: &Pubkey,
    mint_amount: u64,
) -> BanksClientResult<LiquidityProvider> {
    let user = Keypair::new();
    let token_account = Keypair::new();
    let pool_account = Keypair::new();

    let mut lamports: u64 = 0;
    let sol_mint = Pubkey::from_str(SOL_MINT).unwrap();
    if *token_mint_pubkey == sol_mint {
        lamports = mint_amount;
    };

    create_token_account(
        context,
        &token_account,
        // &test_pool.token_mint_pubkey,
        token_mint_pubkey,
        &user.pubkey(),
        lamports,
    )
    .await?;

    create_token_account(
        context,
        &pool_account,
        // &test_pool.pool_mint.pubkey(),
        pool_mint,
        &user.pubkey(),
        0,
    )
    .await?;

    // Fill user account by native token
    transfer(context, &token_account.pubkey(), mint_amount).await?;

    if *token_mint_pubkey != sol_mint {
        mint_tokens(
            context,
            // &test_pool.token_mint_pubkey,
            token_mint_pubkey,
            &token_account.pubkey(),
            mint_amount,
        )
        .await?;
    };

    Ok(LiquidityProvider {
        owner: user,
        token_account: token_account.pubkey(),
        pool_account: pool_account.pubkey(),
    })
}

#[derive(Debug)]
pub struct TokenHolder {
    pub owner: Keypair,
    pub token_account: Pubkey,
}

impl User for TokenHolder {
    fn pubkey(&self) -> Pubkey {
        self.owner.pubkey()
    }
}

pub async fn add_token_holder(
    context: &mut ProgramTestContext,
    token_mint_pubkey: &Pubkey,
    mint_amount: u64,
) -> BanksClientResult<TokenHolder> {
    let user = Keypair::new();
    let token_account = Keypair::new();

    create_token_account(
        context,
        &token_account,
        token_mint_pubkey,
        &user.pubkey(),
        0,
    )
    .await?;

    mint_tokens(
        context,
        token_mint_pubkey,
        &token_account.pubkey(),
        mint_amount,
    )
    .await?;

    Ok(TokenHolder {
        owner: user,
        token_account: token_account.pubkey(),
    })
}
