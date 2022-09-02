/// Process Borrow instruction
pub fn borrow(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let pool_borrow_authority_info = next_account_info(account_info_iter)?;
    let destination_info = next_account_info(account_info_iter)?;
    let token_account_info = next_account_info(account_info_iter)?;
    let pool_market_authority_info = next_account_info(account_info_iter)?;
    let borrow_authority_info = next_account_info(account_info_iter)?;
    let _token_program_info = next_account_info(account_info_iter)?;

    assert_signer(borrow_authority_info)?;

    // Check programs
    assert_owned_by(pool_market_info, program_id)?;
    assert_owned_by(pool_info, program_id)?;
    assert_owned_by(pool_borrow_authority_info, program_id)?;

    let mut pool = Pool::unpack(&pool_info.data.borrow())?;

    // Check pool accounts
    assert_account_key(pool_market_info, &pool.pool_market)?;
    assert_account_key(token_account_info, &pool.token_account)?;

    let mut pool_borrow_authority =
        PoolBorrowAuthority::unpack(&pool_borrow_authority_info.data.borrow())?;

    // Check pool borrow authority accounts
    assert_account_key(pool_info, &pool_borrow_authority.pool)?;
    assert_account_key(
        borrow_authority_info,
        &pool_borrow_authority.borrow_authority,
    )?;

    pool_borrow_authority.borrow(amount)?;
    pool_borrow_authority.check_amount_allowed(total_pool_amount(
        token_account_info.clone(),
        pool.total_amount_borrowed,
    )?)?;
    pool.borrow(amount)?;

    // Check interest ?

    PoolBorrowAuthority::pack(
        pool_borrow_authority,
        *pool_borrow_authority_info.data.borrow_mut(),
    )?;
    Pool::pack(pool, *pool_info.data.borrow_mut())?;

    let (_, bump_seed) = find_program_address(program_id, pool_market_info.key);
    let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

    // Transfer from token account to destination borrower
    cpi::spl_token::transfer(
        token_account_info.clone(),
        destination_info.clone(),
        pool_market_authority_info.clone(),
        amount,
        &[signers_seeds],
    )?;

    Ok(())
}
