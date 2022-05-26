use solana_program_test::{ProgramTestContext};
use solana_sdk::{
    signature::{Signer},
};

use crate::utils::{
    UniversalLiquidityPool,
    UniversalLiquidityPoolBorrowAuthority,
    get_token_balance,
};

pub const ULP_SHARE_ALLOWED: u16 = 10_000;

pub async fn get_amount_allowed(
    context: &mut ProgramTestContext,
    test_pool: &UniversalLiquidityPool,
    test_pool_borrow_authority: &UniversalLiquidityPoolBorrowAuthority,
) -> u64 {
    let token_amount = get_token_balance(context, &test_pool.token_account.pubkey()).await;
    let total_amount_borrowed = test_pool.get_data(context).await.total_amount_borrowed;
    let total_pool_amount = token_amount + total_amount_borrowed;

    test_pool_borrow_authority
        .get_data(context)
        .await
        .get_amount_allowed(total_pool_amount)
        .unwrap()
}