/// Process Deposit instruction
pub fn deposit(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_config_info = next_account_info(account_info_iter)?;
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let source_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let pool_mint_info = next_account_info(account_info_iter)?;
    let pool_market_authority_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    // mining accounts
    let mining_reward_pool = next_account_info(account_info_iter)?;
    let mining_reward_acc = next_account_info(account_info_iter)?;
    let everlend_config = next_account_info(account_info_iter)?;
    let everlend_rewards_program_info = next_account_info(account_info_iter)?;

    assert_owned_by(everlend_config, &eld_config::id())?;
    assert_account_key(everlend_rewards_program_info, &eld_rewards::id())?;

    let _token_program_info = next_account_info(account_info_iter)?;

    assert_signer(user_transfer_authority_info)?;
    assert_owned_by(pool_market_info, program_id)?;
    assert_owned_by(pool_info, program_id)?;

    // Get pool state
    let pool = Pool::unpack(&pool_info.data.borrow())?;

    // Check pool accounts
    assert_account_key(pool_market_info, &pool.pool_market)?;
    assert_account_key(token_account_info, &pool.token_account)?;
    assert_account_key(pool_mint_info, &pool.pool_mint)?;

    let (pool_config_pubkey, _) = find_pool_config_program_address(program_id, pool_info.key);
    assert_account_key(pool_config_info, &pool_config_pubkey)?;

    // Check only if account exists
    if !pool_config_info.owner.eq(&Pubkey::default()) {
        assert_owned_by(pool_config_info, program_id)?;

        let pool_config = PoolConfig::unpack(&pool_config_info.data.borrow())?;
        if amount < pool_config.deposit_minimum {
            return Err(EverlendError::DepositAmountTooSmall.into());
        }
    }

    let total_incoming =
        total_pool_amount(token_account_info.clone(), pool.total_amount_borrowed)?;
    let total_minted = Mint::unpack_unchecked(&pool_mint_info.data.borrow())?.supply;

    let mint_amount = if total_incoming == 0 || total_minted == 0 {
        amount
    } else {
        (amount as u128)
            .checked_mul(total_minted as u128)
            .ok_or(ProgramError::InvalidArgument)?
            .checked_div(total_incoming as u128)
            .ok_or(ProgramError::InvalidArgument)? as u64
    };

    if mint_amount == 0 {
        return Err(EverlendError::DepositAmountTooSmall.into());
    }

    // Transfer token from source to token account
    cpi::spl_token::transfer(
        source_info.clone(),
        token_account_info.clone(),
        user_transfer_authority_info.clone(),
        amount,
        &[],
    )?;

    let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
    let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

    // Mint to destination pool token
    cpi::spl_token::mint_to(
        pool_mint_info.clone(),
        destination_info.clone(),
        pool_market_authority_info.clone(),
        mint_amount,
        &[signers_seeds],
    )?;

    let (pool_pubkey, pool_bump_seed) =
        find_pool_program_address(program_id, &pool.pool_market, &pool.token_mint);
    assert_account_key(pool_info, &pool_pubkey)?;

    let pool_seeds: &[&[u8]] = &[
        &pool.pool_market.to_bytes()[..32],
        &pool.token_mint.to_bytes()[..32],
        &[pool_bump_seed],
    ];

    assert_owned_by(mining_reward_pool, &eld_rewards::id())?;
    assert_owned_by(mining_reward_acc, &eld_rewards::id())?;

    deposit_mining(
        everlend_rewards_program_info.key,
        everlend_config.clone(),
        mining_reward_pool.clone(),
        mining_reward_acc.clone(),
        user_transfer_authority_info.clone(),
        pool_info.to_owned(),
        mint_amount,
        &[pool_seeds],
    )?;

    Ok(())
}

