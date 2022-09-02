  /// Process Withdraw instruction
  pub fn withdraw(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_market_authority_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let pool_mint_info = next_account_info(account_info_iter)?;
    let withdrawal_requests_info = next_account_info(account_info_iter)?;
    let withdrawal_request_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let collateral_transit_info = next_account_info(account_info_iter)?;
    let from_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;
    let clock = Clock::from_account_info(clock_info)?;
    let _token_program_info = next_account_info(account_info_iter)?;

    // Check programs
    assert_owned_by(pool_market_info, program_id)?;
    assert_owned_by(pool_info, program_id)?;
    assert_owned_by(withdrawal_requests_info, program_id)?;
    assert_owned_by(withdrawal_request_info, program_id)?;

    // Check collateral token transit account
    let (collateral_transit_pubkey, _) =
        find_transit_program_address(program_id, pool_market_info.key, pool_mint_info.key);
    assert_account_key(collateral_transit_info, &collateral_transit_pubkey)?;

    // We don't check the pool pda, because it's created from the program
    // and is linked to the pool market

    // Get pool state
    let pool = Pool::unpack(&pool_info.data.borrow())?;
    assert_account_key(pool_market_info, &pool.pool_market)?;
    assert_account_key(token_account_info, &pool.token_account)?;
    assert_account_key(pool_mint_info, &pool.pool_mint)?;

    // We don't check the withdrawal requests pda, because it's created from the program
    // and is linked to the pool

    let mut withdrawal_requests =
        WithdrawalRequests::unpack(&withdrawal_requests_info.data.borrow())?;
    assert_account_key(pool_info, &withdrawal_requests.pool)?;

    let withdrawal_request = WithdrawalRequest::unpack(&withdrawal_request_info.data.borrow())?;

    // Check withdraw request accounts
    assert_account_key(pool_info, &withdrawal_request.pool)?;
    assert_account_key(destination_info, &withdrawal_request.destination)?;
    assert_account_key(from_info, &withdrawal_request.from)?;

    // Check that enough time has passed to make a withdraw
    if withdrawal_request.ticket > clock.slot {
        return Err(EverlendError::WithdrawRequestsInvalidTicket.into());
    }

    let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
    let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

    // In the case of a SOL token, we do unwrap SPL token,
    // the destination can be any account

    if pool.token_mint == spl_token::native_mint::id() {
        let token_mint_info = next_account_info(account_info_iter)?;
        assert_account_key(token_mint_info, &pool.token_mint)?;

        let unwrap_sol_info = next_account_info(account_info_iter)?;

        // Check transit: unwrapped sol
        let (unwrap_sol_pubkey, bump_seed) =
            find_transit_sol_unwrap_address(program_id, withdrawal_request_info.key);
        assert_account_key(unwrap_sol_info, &unwrap_sol_pubkey)?;

        let signer_info = next_account_info(account_info_iter)?;

        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_info = next_account_info(account_info_iter)?;

        let unwrap_acc_signers_seeds = &[
            br"unwrap",
            &withdrawal_request_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        cpi::system::create_account::<spl_token::state::Account>(
            &spl_token::id(),
            signer_info.clone(),
            unwrap_sol_info.clone(),
            &[unwrap_acc_signers_seeds],
            rent,
        )?;

        cpi::spl_token::initialize_account(
            unwrap_sol_info.clone(),
            token_mint_info.clone(),
            pool_market_authority_info.clone(),
            rent_info.clone(),
        )?;

        // Transfer from token account to destination
        cpi::spl_token::transfer(
            token_account_info.clone(),
            unwrap_sol_info.clone(),
            pool_market_authority_info.clone(),
            withdrawal_request.liquidity_amount,
            &[signers_seeds],
        )?;

        cpi::spl_token::close_account(
            signer_info.clone(),
            unwrap_sol_info.clone(),
            pool_market_authority_info.clone(),
            &[signers_seeds],
        )?;

        cpi::system::transfer(
            signer_info.clone(),
            destination_info.clone(),
            withdrawal_request.liquidity_amount,
            &[],
        )?;
    } else {
        // Transfer from token account to destination
        cpi::spl_token::transfer(
            token_account_info.clone(),
            destination_info.clone(),
            pool_market_authority_info.clone(),
            withdrawal_request.liquidity_amount,
            &[signers_seeds],
        )?;
    };

    // Burn from transit collateral pool token
    cpi::spl_token::burn(
        pool_mint_info.clone(),
        collateral_transit_info.clone(),
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
        Default::default(),
        *withdrawal_request_info.data.borrow_mut(),
    )?;

    Ok(())
}

