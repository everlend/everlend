/// Process UpdateManager instruction
pub fn set_token_metadata(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    symbol: String,
    uri: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_market_info = next_account_info(account_info_iter)?;
    let pool_info = next_account_info(account_info_iter)?;
    let pool_mint_info = next_account_info(account_info_iter)?;
    let pool_market_authority_info = next_account_info(account_info_iter)?;
    let metadata_info = next_account_info(account_info_iter)?;
    let manager_info = next_account_info(account_info_iter)?;
    let metaplex_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    // Check manager
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
    assert_account_key(pool_mint_info, &pool.pool_mint)?;

    // Get authority
    let (pool_market_authority, bump_seed) =
        find_program_address(program_id, pool_market_info.key);
    assert_account_key(pool_market_authority_info, &pool_market_authority)?;

    let signers_seeds = &[&pool_market_info.key.to_bytes()[..32], &[bump_seed]];

    if metadata_info.owner.eq(&Pubkey::default()) {
        create_metadata(
            metaplex_program_info.clone(),
            metadata_info.clone(),
            pool_mint_info.clone(),
            pool_market_authority_info.clone(),
            manager_info.clone(),
            system_program_info.clone(),
            rent_info.clone(),
            name,
            symbol,
            uri,
            &[signers_seeds],
        )?;
    } else {
        update_metadata(
            metaplex_program_info.clone(),
            metadata_info.clone(),
            pool_mint_info.clone(),
            pool_market_authority_info.clone(),
            name,
            symbol,
            uri,
            &[signers_seeds],
        )?;
    }

    Ok(())
}

