use crate::utils::{LiquidityProvider, BanksClientResult, create_token_account, mint_tokens, transfer};

use solana_program::{pubkey::Pubkey};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

pub async fn add_liquidity_provider(
    context: &mut ProgramTestContext,
    token_mint_pubkey: &Pubkey,
    mint_amount: u64,
) -> BanksClientResult<LiquidityProvider> {
    let user = Keypair::new();
    let token_account = Keypair::new();
    let pool_account = Keypair::new();

    let mut lamports: u64 = 0;
    if *token_mint_pubkey == spl_token::native_mint::id() {
        lamports = mint_amount;
    };

    create_token_account(
        context,
        &token_account,
        token_mint_pubkey,
        &user.pubkey(),
        lamports,
    )
    .await?;


    if *token_mint_pubkey != spl_token::native_mint::id() {
        mint_tokens(
            context,
            // &test_pool.token_mint_pubkey,
            token_mint_pubkey,
            &token_account.pubkey(),
            mint_amount,
        )
        .await?;
    } else {
        // Fill user account by native token
        transfer(context, &token_account.pubkey(), mint_amount).await?;
    }

    Ok(LiquidityProvider {
        owner: user,
        token_account: token_account.pubkey(),
        pool_account: pool_account.pubkey(),
    })
}

