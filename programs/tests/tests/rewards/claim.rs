use crate::utils::*;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program_test::*;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use spl_token::state::Account;
use std::borrow::Borrow;

async fn setup() -> (
    ProgramTestContext,
    TestRewards,
    Keypair,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
) {
    let mut env = presetup().await;
    let owner = &env.context.payer.pubkey();

    let mint = Keypair::new();
    create_mint(&mut env.context, &mint, &owner).await.unwrap();

    let test_reward_pool = TestRewards::new(Some(mint.pubkey()));
    test_reward_pool
        .initialize_pool(&mut env.context)
        .await
        .unwrap();

    let user = Keypair::new();
    let user_mining = test_reward_pool
        .initialize_mining(&mut env.context, &user.pubkey())
        .await;
    test_reward_pool
        .deposit_mining(&mut env.context, &user.pubkey(), &user_mining, 100)
        .await
        .unwrap();

    let rewarder = Keypair::new();
    create_token_account(&mut env.context, &rewarder, &mint.pubkey(), owner, 0)
        .await
        .unwrap();
    mint_tokens(
        &mut env.context,
        &mint.pubkey(),
        &rewarder.pubkey(),
        1_000_000,
    )
        .await
        .unwrap();

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

    test_reward_pool
        .add_vault(&mut env.context, &fee_keypair.pubkey(), &test_reward_pool.token_mint_pubkey)
        .await;

    let subreward_mint = Keypair::new();
    create_mint(&mut env.context, &subreward_mint, &owner).await.unwrap();

    let sub_rewarder = Keypair::new();
    create_token_account(&mut env.context, &sub_rewarder, &subreward_mint.pubkey(), owner, 0)
        .await
        .unwrap();

    mint_tokens(
        &mut env.context,
        &subreward_mint.pubkey(),
        &sub_rewarder.pubkey(),
        1_000_000,
    )
        .await
        .unwrap();

    let sub_fee_keypair = Keypair::new();
    create_token_account(
        &mut env.context,
        &sub_fee_keypair,
        &subreward_mint.pubkey(),
        &user.pubkey(),
        0,
    )
        .await
        .unwrap();

    test_reward_pool
        .add_vault(&mut env.context, &sub_fee_keypair.pubkey(), &subreward_mint.pubkey())
        .await;

    (
        env.context,
        test_reward_pool,
        user,
        user_mining,
        fee_keypair.pubkey(),
        rewarder.pubkey(),
        sub_fee_keypair.pubkey(),
        sub_rewarder.pubkey(),
        subreward_mint.pubkey()
    )
}

#[tokio::test]
async fn success() {
    let (mut context, test_rewards, user, user_mining, fee, rewarder, sub_fee_keypair, sub_rewarder, subreward_mint) = setup().await;

    test_rewards
        .fill_vault(&mut context, &fee, &rewarder, &test_rewards.token_mint_pubkey, 1_000_000)
        .await
        .unwrap();

    test_rewards
        .fill_vault(&mut context, &sub_fee_keypair, &sub_rewarder, &subreward_mint, 500_000)
        .await
        .unwrap();

    let user_reward_main = Keypair::new();
    create_token_account(
        &mut context,
        &user_reward_main,
        &test_rewards.token_mint_pubkey,
        &user.pubkey(),
        0,
    )
        .await
        .unwrap();

    let user_reward_sub = Keypair::new();
    create_token_account(
        &mut context,
        &user_reward_sub,
        &subreward_mint,
        &user.pubkey(),
        0,
    )
        .await
        .unwrap();

    test_rewards
        .claim(&mut context, &user, &user_mining, vec![user_reward_main.pubkey(), user_reward_sub.pubkey()])
        .await
        .unwrap();

    let user_reward_account_main = get_account(&mut context, &user_reward_main.pubkey()).await;
    let user_reward_main = Account::unpack(user_reward_account_main.data.borrow()).unwrap();

    let user_reward_account_sub = get_account(&mut context, &user_reward_sub.pubkey()).await;
    let user_reward_sub = Account::unpack(user_reward_account_sub.data.borrow()).unwrap();

    assert_eq!(user_reward_main.amount, 980_000);
    assert_eq!(user_reward_sub.amount, 490_000);
}

#[tokio::test]
async fn with_two_users() {
    let (mut context, test_rewards, user1, user_mining1, fee, rewarder, sub_fee_keypair, sub_rewarder, subreward_mint) = setup().await;

    let user2 = Keypair::new();
    let user_mining2 = test_rewards
        .initialize_mining(&mut context, &user2.pubkey())
        .await;
    test_rewards
        .deposit_mining(&mut context, &user2.pubkey(), &user_mining2, 50)
        .await
        .unwrap();

    test_rewards
        .fill_vault(&mut context, &fee, &rewarder, &test_rewards.token_mint_pubkey,1_000_000)
        .await
        .unwrap();

    test_rewards
        .fill_vault(&mut context, &sub_fee_keypair, &sub_rewarder, &subreward_mint, 750_000)
        .await
        .unwrap();

    let user1_reward_main = Keypair::new();
    create_token_account(
        &mut context,
        &user1_reward_main,
        &test_rewards.token_mint_pubkey,
        &user1.pubkey(),
        0,
    )
        .await
        .unwrap();

    let user1_reward_sub = Keypair::new();
    create_token_account(
        &mut context,
        &user1_reward_sub,
        &subreward_mint,
        &user1.pubkey(),
        0,
    )
        .await
        .unwrap();

    test_rewards
        .claim(&mut context, &user1, &user_mining1, vec![user1_reward_main.pubkey(), user1_reward_sub.pubkey()])
        .await
        .unwrap();

    let user2_reward_main = Keypair::new();
    create_token_account(
        &mut context,
        &user2_reward_main,
        &test_rewards.token_mint_pubkey,
        &user2.pubkey(),
        0,
    )
        .await
        .unwrap();

    let user2_reward_sub = Keypair::new();
    create_token_account(
        &mut context,
        &user2_reward_sub,
        &subreward_mint,
        &user2.pubkey(),
        0,
    )
        .await
        .unwrap();

    test_rewards
        .claim(&mut context, &user2, &user_mining2, vec![user2_reward_main.pubkey(), user2_reward_sub.pubkey()])
        .await
        .unwrap();

    let user_reward_account1_main = get_account(&mut context, &user1_reward_main.pubkey()).await;
    let user_reward1_main = Account::unpack(user_reward_account1_main.data.borrow()).unwrap();

    let user_reward_account1_sub = get_account(&mut context, &user1_reward_sub.pubkey()).await;
    let user_reward1_sub = Account::unpack(user_reward_account1_sub.data.borrow()).unwrap();

    assert_eq!(user_reward1_main.amount, 653_333);
    assert_eq!(user_reward1_sub.amount, 490_000);

    let user_reward_account2_main = get_account(&mut context, &user2_reward_main.pubkey()).await;
    let user_reward2_main = Account::unpack(user_reward_account2_main.data.borrow()).unwrap();

    let user_reward_account2_sub = get_account(&mut context, &user2_reward_sub.pubkey()).await;
    let user_reward2_sub = Account::unpack(user_reward_account2_sub.data.borrow()).unwrap();

    assert_eq!(user_reward2_main.amount, 326_666);
    assert_eq!(user_reward2_sub.amount, 245_000);
}