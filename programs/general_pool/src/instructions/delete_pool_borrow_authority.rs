 /// Process DeletePoolBorrowAuthority instruction
 pub fn delete_pool_borrow_authority(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let pool_borrow_authority_info = next_account_info(account_info_iter)?;
    let receiver_info = next_account_info(account_info_iter)?;
    let manager_info = next_account_info(account_info_iter)?;

    assert_signer(manager_info)?;

    // Check programs
    assert_owned_by(pool_market_info, program_id)?;
    assert_owned_by(pool_info, program_id)?;
    assert_owned_by(pool_borrow_authority_info, program_id)?;

    let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;

    // Check manager
    assert_account_key(manager_info, &pool_market.manager)?;

    let pool = Pool::unpack(&pool_info.data.borrow())?;

    // Check pool accounts
    assert_account_key(pool_market_info, &pool.pool_market)?;

    // Get pool borrow authority state to check initialized
    let pool_borrow_authority =
        PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;
    assert_account_key(pool_info, &pool_borrow_authority.pool)?;

    let receiver_starting_lamports = receiver_info.lamports();
    let pool_borrow_authority_lamports = pool_borrow_authority_info.lamports();

    **pool_borrow_authority_info.lamports.borrow_mut() = 0;
    **receiver_info.lamports.borrow_mut() = receiver_starting_lamports
        .checked_add(pool_borrow_authority_lamports)
        .ok_or(EverlendError::MathOverflow)?;

    PoolBorrowAuthority::pack(
        Default::default(),
        *pool_borrow_authority_info.data.borrow_mut(),
    )?;

    Ok(())
}

