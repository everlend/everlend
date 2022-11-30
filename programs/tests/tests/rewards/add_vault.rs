use crate::utils::*;
use everlend_rewards::state::RewardPool;
use solana_program::program_pack::Pack;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::borrow::Borrow;

async fn setup() -> (ProgramTestContext, TestRewards, Keypair) {
    let mut env = presetup().await;

    let test_reward_pool = TestRewards::new(None);

    test_reward_pool
        .initialize_pool(&mut env.context)
        .await
        .unwrap();

    let user = Keypair::new();
    let fee_keypair = Keypair::new();

    create_token_account(
        &mut env.context,
        &fee_keypair,
        &test_reward_pool.token_mint_pubkey,
        &user.pubkey(),
        0,
    )
    .await
    .unwrap();

    (env.context, test_reward_pool, fee_keypair)
}

#[tokio::test]
async fn success() {
    let (mut context, test_rewards, fee_keypair) = setup().await;

    test_rewards
        .add_vault(&mut context, &fee_keypair.pubkey(), &test_rewards.token_mint_pubkey)
        .await;

    let reward_pool_account = get_account(&mut context, &test_rewards.mining_reward_pool).await;
    let reward_pool = RewardPool::unpack(reward_pool_account.data.borrow()).unwrap();
    let vaults = reward_pool.vaults.get(0).unwrap();

    assert_eq!(vaults.fee_account, fee_keypair.pubkey());
    assert_eq!(vaults.reward_mint, test_rewards.token_mint_pubkey);
}
