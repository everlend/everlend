use std::borrow::Borrow;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::{
    signer::Signer
};
use solana_sdk::signature::Keypair;
use everlend_rewards::state::{Mining, RewardPool};
use crate::utils::*;

async fn setup() -> (
    ProgramTestContext,
    TestRewards,
    Pubkey,
    Pubkey,
) {
    let mut env = presetup().await;

    let test_reward_pool = TestRewards::new(None);
    test_reward_pool
        .initialize_pool(&mut env.context)
        .await
        .unwrap();

    let user = Keypair::new();
    let user_mining = test_reward_pool
        .initialize_mining(&mut env.context, &user.pubkey())
        .await;

    (env.context, test_reward_pool, user.pubkey(), user_mining)
}

#[tokio::test]
async fn success() {
    let (mut context, test_rewards, user, mining) = setup().await;

    test_rewards
        .deposit_mining(&mut context, &user, &mining, 100)
        .await
        .unwrap();

    test_rewards
        .withdraw_mining(&mut context, &user, &mining, 30)
        .await
        .unwrap();

    let reward_pool_account = get_account(&mut context, &test_rewards.mining_reward_pool)
        .await;
    let reward_pool = RewardPool::unpack(reward_pool_account.data.borrow()).unwrap();

    assert_eq!(reward_pool.total_share, 70);

    let mining_account = get_account(&mut context, &mining).await;
    let mining = Mining::unpack(&mining_account.data.borrow()).unwrap();
    assert_eq!(mining.share, 70);
}