  /// Process UpdatePoolBorrowAuthority instruction
  pub fn update_pool_borrow_authority(
    program_id: &Pubkey,
    share_allowed: u16,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let pool_borrow_authority_info = next_account_info(account_info_iter)?;
    let manager_info = next_account_info(account_info_iter)?;

    assert_signer(manager_info)?;

    // Check programs
    assert_owned_by(pool_market_info, program_id)?;
    assert_owned_by(pool_info, program_id)?;
    assert_owned_by(pool_borrow_authority_info, program_id)?;

    // Get pool market state
    let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
    assert_account_key(manager_info, &pool_market.manager)?;

    // Get pool state
    let pool = Pool::unpack(&pool_info.data.borrow())?;
    assert_account_key(pool_market_info, &pool.pool_market)?;

    // Get pool borrow authority state
    let mut pool_borrow_authority =
        PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;
    assert_account_key(pool_info, &pool_borrow_authority.pool)?;

    pool_borrow_authority.update_share_allowed(share_allowed);

    PoolBorrowAuthority::pack(
        pool_borrow_authority,
        *pool_borrow_authority_info.data.borrow_mut(),
    )?;

    Ok(())
}

