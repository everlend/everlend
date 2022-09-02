/// Process Cancel withdraw request instruction
pub fn cancel_withdraw_request(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let withdrawal_requests_info = next_account_info(account_info_iter)?;
    let withdrawal_request_info = next_account_info(account_info_iter)?;
    let source_info = next_account_info(account_info_iter)?;
    let collateral_transit_info = next_account_info(account_info_iter)?;
    let pool_mint_info = next_account_info(account_info_iter)?;
    let pool_market_authority_info = next_account_info(account_info_iter)?;
    let from_info = next_account_info(account_info_iter)?;
    let manager_info = next_account_info(account_info_iter)?;
    let _token_program_info = next_account_info(account_info_iter)?;

    assert_signer(manager_info)?;

    // Check programs
    assert_owned_by(pool_market_info, program_id)?;
    assert_owned_by(pool_info, program_id)?;
    assert_owned_by(withdrawal_requests_info, program_id)?;
    assert_owned_by(withdrawal_request_info, program_id)?;

    let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;

    // Check manager
    assert_account_key(manager_info, &pool_market.manager)?;

    // Check collateral token transit account
    let (collateral_transit_pubkey, _) =
        find_transit_program_address(program_id, pool_market_info.key, pool_mint_info.key);
    assert_account_key(collateral_transit_info, &collateral_transit_pubkey)?;

    // We don't check the pool pda, because it's created from the program
    // and is linked to the pool market

    // Get pool state
    let pool = Pool::unpack(&pool_info.data.borrow())?;
    assert_account_key(pool_market_info, &pool.pool_market)?;
    assert_account_key(pool_mint_info, &pool.pool_mint)?;

    // We don't check the withdrawal requests pda, because it's created from the program
    // and is linked to the pool

    let mut withdrawal_requests =
        WithdrawalRequests::unpack(&withdrawal_requests_info.data.borrow())?;

    // Check withdrawal requests accounts
    assert_account_key(pool_info, &withdrawal_requests.pool)?;

    let withdrawal_request = WithdrawalRequest::unpack(&withdrawal_request_info.data.borrow())?;

    // Check withdrawal request accounts
    assert_account_key(pool_info, &withdrawal_request.pool)?;
    assert_account_key(source_info, &withdrawal_request.source)?;
    assert_account_key(from_info, &withdrawal_request.from)?;

    let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
    let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

    cpi::spl_token::transfer(
        collateral_transit_info.clone(),
        source_info.clone(),
        pool_market_authority_info.clone(),
        withdrawal_request.collateral_amount,
        &[signers_seeds],
    )?;

    withdrawal_requests.process(withdrawal_request.liquidity_amount)?;

    // Close withdraw account and return rent
    let from_starting_lamports = from_info.lamports();
    let withdraw_request_lamports = withdrawal_request_info.lamports();

    **withdrawal_request_info.lamports.borrow_mut() = 0;
    **from_info.lamports.borrow_mut() = from_starting_lamports
        .checked_add(withdraw_request_lamports)
        .ok_or(EverlendError::MathOverflow)?;

    WithdrawalRequests::pack(
        withdrawal_requests,
        *withdrawal_requests_info.data.borrow_mut(),
    )?;
    WithdrawalRequest::pack(
        withdrawal_request,
        *withdrawal_request_info.data.borrow_mut(),
    )?;

    Ok(())
}

