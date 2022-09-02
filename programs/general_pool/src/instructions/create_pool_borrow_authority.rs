/// Process CreatePoolBorrowAuthority instruction
pub fn create_pool_borrow_authority(
    program_id: &Pubkey,
    share_allowed: u16,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let pool_borrow_authority_info = next_account_info(account_info_iter)?;
    let borrow_authority_info = next_account_info(account_info_iter)?;
    let manager_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    assert_signer(manager_info)?;

    // Check programs
    assert_owned_by(pool_market_info, program_id)?;
    assert_owned_by(pool_info, program_id)?;

    // Get pool market state
    let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
    assert_account_key(manager_info, &pool_market.manager)?;

    // Get pool state
    let pool = Pool::unpack(&pool_info.data.borrow())?;
    assert_account_key(pool_market_info, &pool.pool_market)?;

    // Create pool borrow authority account
    let (pool_borrow_authority_pubkey, bump_seed) = find_pool_borrow_authority_program_address(
        program_id,
        pool_info.key,
        borrow_authority_info.key,
    );
    assert_account_key(pool_borrow_authority_info, &pool_borrow_authority_pubkey)?;

    let signers_seeds = &[
        &pool_info.key.to_bytes()[..32],
        &borrow_authority_info.key.to_bytes()[..32],
        &[bump_seed],
    ];

    cpi::system::create_account::<PoolBorrowAuthority>(
        program_id,
        manager_info.clone(),
        pool_borrow_authority_info.clone(),
        &[signers_seeds],
        rent,
    )?;

    // Get pool borrow authority state
    let mut pool_borrow_authority =
        PoolBorrowAuthority::unpack_unchecked(&pool_borrow_authority_info.data.borrow())?;
    assert_uninitialized(&pool_borrow_authority)?;

    pool_borrow_authority.init(InitPoolBorrowAuthorityParams {
        pool: *pool_info.key,
        borrow_authority: *borrow_authority_info.key,
        share_allowed,
    });

    PoolBorrowAuthority::pack(
        pool_borrow_authority,
        *pool_borrow_authority_info.data.borrow_mut(),
    )?;

    Ok(())
}

