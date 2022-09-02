 /// Migrate withdraw request
 pub fn init_user_mining(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let user_collateral_token_account_info = next_account_info(account_info_iter)?;
    let user_authority = next_account_info(account_info_iter)?;
    let registry_info = next_account_info(account_info_iter)?;
    let manager_info = next_account_info(account_info_iter)?;
    let mining_reward_pool = next_account_info(account_info_iter)?;
    let mining_reward_acc = next_account_info(account_info_iter)?;

    let everlend_config = next_account_info(account_info_iter)?;
    let everlend_rewards_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    assert_signer(manager_info)?;
    assert_owned_by(registry_info, &everlend_registry::id())?;

    let registry = Registry::unpack(&registry_info.data.borrow())?;
    assert_account_key(manager_info, &registry.manager)?;

    assert_owned_by(pool_market_info, program_id)?;
    assert_owned_by(pool_info, program_id)?;

    assert_owned_by(everlend_config, &eld_config::id())?;
    assert_owned_by(mining_reward_pool, &eld_rewards::id())?;
    assert_account_key(everlend_rewards_program_info, &eld_rewards::id())?;

    let pool = Pool::unpack(&pool_info.data.borrow())?;
    assert_account_key(pool_market_info, &pool.pool_market)?;

    let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
    assert_account_key(registry_info, &pool_market.registry)?;

    let (pool_pubkey, pool_bump_seed) =
        find_pool_program_address(program_id, &pool.pool_market, &pool.token_mint);
    assert_account_key(pool_info, &pool_pubkey)?;

    let pool_seeds: &[&[u8]] = &[
        &pool.pool_market.to_bytes()[..32],
        &pool.token_mint.to_bytes()[..32],
        &[pool_bump_seed],
    ];

    let user_account = Account::unpack(&user_collateral_token_account_info.data.borrow())?;
    if pool.pool_mint != user_account.mint {
        return Err(ProgramError::InvalidArgument);
    }

    // check authority
    if !user_account.owner.eq(user_authority.key) {
        return Err(ProgramError::InvalidArgument);
    }

    if !mining_reward_acc.owner.eq(&Pubkey::default()) {
        return Err(ProgramError::InvalidArgument);
    }

    initialize_mining(
        everlend_rewards_program_info.key,
        everlend_config.clone(),
        mining_reward_pool.clone(),
        mining_reward_acc.clone(),
        user_authority.clone(),
        manager_info.clone(),
        system_program_info.clone(),
        rent_info.clone(),
    )?;

    deposit_mining(
        everlend_rewards_program_info.key,
        everlend_config.clone(),
        mining_reward_pool.clone(),
        mining_reward_acc.clone(),
        user_authority.clone(),
        pool_info.to_owned(),
        user_account.amount,
        &[pool_seeds],
    )?;

    Ok(())
}