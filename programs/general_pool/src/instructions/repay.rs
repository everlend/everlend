 /// Process Repay instruction
 pub fn repay(
    program_id: &Pubkey,
    amount: u64,
    interest_amount: u64,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let pool_borrow_authority_info = next_account_info(account_info_iter)?;
    let source_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    let _token_program_info = next_account_info(account_info_iter)?;

    assert_signer(user_transfer_authority_info)?;

    // Check programs
    assert_owned_by(pool_market_info, program_id)?;
    assert_owned_by(pool_info, program_id)?;
    assert_owned_by(pool_borrow_authority_info, program_id)?;

    // Get pool state
    let mut pool = Pool::unpack(&pool_info.data.borrow())?;

    // Check pool accounts
    assert_account_key(pool_market_info, &pool.pool_market)?;
    assert_account_key(token_account_info, &pool.token_account)?;

    // Get pool borrow authority state
    let mut pool_borrow_authority =
        PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;
    assert_account_key(pool_info, &pool_borrow_authority.pool)?;

    pool_borrow_authority.repay(amount)?;
    pool.repay(amount)?;

    // Check interest ?

    PoolBorrowAuthority::pack(
        pool_borrow_authority,
        *pool_borrow_authority_info.data.borrow_mut(),
    )?;
    Pool::pack(pool, *pool_info.data.borrow_mut())?;

    // Transfer from source to token account
    cpi::spl_token::transfer(
        source_info.clone(),
        token_account_info.clone(),
        user_transfer_authority_info.clone(),
        amount + interest_amount,
        &[],
    )?;

    Ok(())
}

