/// Process Withdraw request instruction
pub fn withdraw_request(
    program_id: &Pubkey,
    collateral_amount: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_config_info = next_account_info(account_info_iter)?;
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let pool_mint_info = next_account_info(account_info_iter)?;
    let withdrawal_requests_info = next_account_info(account_info_iter)?;
    let withdrawal_request_info = next_account_info(account_info_iter)?;
    let source_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let collateral_transit_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    // mining accounts
    let mining_reward_pool = next_account_info(account_info_iter)?;
    let mining_reward_acc = next_account_info(account_info_iter)?;
    let everlend_config = next_account_info(account_info_iter)?;
    let everlend_rewards_program_info = next_account_info(account_info_iter)?;

    assert_owned_by(everlend_config, &eld_config::id())?;
    assert_account_key(everlend_rewards_program_info, &eld_rewards::id())?;

    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;
    let clock_info = next_account_info(account_info_iter)?;
    let clock = Clock::from_account_info(clock_info)?;
    let _system_program_info = next_account_info(account_info_iter)?;
    let _token_program_info = next_account_info(account_info_iter)?;

    assert_signer(user_transfer_authority_info)?;

    assert_owned_by(pool_market_info, program_id)?;
    assert_owned_by(pool_info, program_id)?;
    assert_owned_by(withdrawal_requests_info, program_id)?;

    let pool = Pool::unpack(&pool_info.data.borrow())?;

    // Check pool accounts
    assert_account_key(pool_market_info, &pool.pool_market)?;
    assert_account_key(token_account_info, &pool.token_account)?;
    assert_account_key(pool_mint_info, &pool.pool_mint)?;

    // In all cases except SOL token, we must check destination account
    if pool.token_mint != spl_token::native_mint::id() {
        let destination_account = Account::unpack(&destination_info.data.borrow())?;
        if pool.token_mint != destination_account.mint {
            return Err(ProgramError::InvalidArgument);
        }
    }

    // Check transit: collateral
    let (collateral_transit_pubkey, _) =
        find_transit_program_address(program_id, pool_market_info.key, pool_mint_info.key);
    assert_account_key(collateral_transit_info, &collateral_transit_pubkey)?;

    let mut withdrawal_requests =
        WithdrawalRequests::unpack(&withdrawal_requests_info.data.borrow())?;

    // Check withdrawal requests accounts
    assert_account_key(pool_info, &withdrawal_requests.pool)?;

    // Check withdrawal request
    let (withdrawal_request_pubkey, bump_seed) = find_withdrawal_request_program_address(
        program_id,
        withdrawal_requests_info.key,
        user_transfer_authority_info.key,
    );
    assert_account_key(withdrawal_request_info, &withdrawal_request_pubkey)?;

    let total_incoming =
        total_pool_amount(token_account_info.clone(), pool.total_amount_borrowed)?;
    let total_minted = Mint::unpack_unchecked(&pool_mint_info.data.borrow())?.supply;

    let liquidity_amount = (collateral_amount as u128)
        .checked_mul(total_incoming as u128)
        .ok_or(EverlendError::MathOverflow)?
        .checked_div(total_minted as u128)
        .ok_or(EverlendError::MathOverflow)? as u64;

    let (pool_config_pubkey, _) = find_pool_config_program_address(program_id, pool_info.key);
    assert_account_key(pool_config_info, &pool_config_pubkey)?;

    if !pool_config_info.owner.eq(&Pubkey::default()) {
        assert_owned_by(pool_config_info, program_id)?;

        let pool_config = PoolConfig::unpack(&pool_config_info.data.borrow())?;
        if liquidity_amount < pool_config.withdraw_minimum {
            return Err(EverlendError::WithdrawAmountTooSmall.into());
        }
    }

    // Transfer
    cpi::spl_token::transfer(
        source_info.clone(),
        collateral_transit_info.clone(),
        user_transfer_authority_info.clone(),
        collateral_amount,
        &[],
    )?;

    let signers_seeds = &[
        br"withdrawal",
        &withdrawal_requests_info.key.to_bytes()[..32],
        &user_transfer_authority_info.key.to_bytes()[..32],
        &[bump_seed],
    ];

    cpi::system::create_account::<WithdrawalRequest>(
        program_id,
        user_transfer_authority_info.clone(),
        withdrawal_request_info.clone(),
        &[signers_seeds],
        rent,
    )?;

    let mut withdrawal_request =
        WithdrawalRequest::unpack_unchecked(&withdrawal_request_info.data.borrow())?;

    withdrawal_request.init(InitWithdrawalRequestParams {
        pool: *pool_info.key,
        from: *user_transfer_authority_info.key,
        source: *source_info.key,
        destination: *destination_info.key,
        liquidity_amount,
        collateral_amount,
        ticket: clock.slot + WITHDRAW_DELAY,
    });

    withdrawal_requests.add(liquidity_amount)?;

    WithdrawalRequests::pack(
        withdrawal_requests,
        *withdrawal_requests_info.data.borrow_mut(),
    )?;
    WithdrawalRequest::pack(
        withdrawal_request,
        *withdrawal_request_info.data.borrow_mut(),
    )?;

    // Mining reward
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

    withdraw_mining(
        everlend_rewards_program_info.key,
        everlend_config.clone(),
        mining_reward_pool.clone(),
        mining_reward_acc.clone(),
        user_transfer_authority_info.clone(),
        pool_info.to_owned(),
        collateral_amount,
        &[pool_seeds],
    )?;

    Ok(())
}

