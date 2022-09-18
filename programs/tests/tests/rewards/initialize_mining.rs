use std::borrow::Borrow;
use solana_program::program_pack::Pack;
use solana_program_test::*;
use solana_sdk::{
    signer::Signer
};
use solana_sdk::signature::Keypair;
use everlend_rewards::state::{Mining};
use crate::utils::*;

async fn setup() -> (
    ProgramTestContext,
    TestRewards,
) {
    let mut env = presetup().await;
    let test_reward_pool = TestRewards::new(None);

    test_reward_pool
        .initialize_pool(&mut env.context)
        .await
        .unwrap();

    (env.context, test_reward_pool)
}

#[tokio::test]
async fn success() {
    let (mut context, test_rewards) = setup().await;

    let user = Keypair::new();
    let user_mining = test_rewards
        .initialize_mining(&mut context, &user.pubkey())
        .await;

    let mining_account = get_account(&mut context, &user_mining).await;
    let mining = Mining::unpack(&mining_account.data.borrow()).unwrap();

    assert_eq!(mining.reward_pool, test_rewards.mining_reward_pool);
    assert_eq!(mining.owner, user.pubkey());
}
