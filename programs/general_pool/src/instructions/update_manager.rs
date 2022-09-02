/// Process UpdateManager instruction
pub fn update_manager(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let pool_market_info = next_account_info(account_info_iter)?;
    let manager_info = next_account_info(account_info_iter)?;
    let new_manager_info = next_account_info(account_info_iter)?;

    assert_signer(manager_info)?;
    assert_signer(new_manager_info)?;

    assert_owned_by(pool_market_info, program_id)?;

    let mut pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
    assert_account_key(manager_info, &pool_market.manager)?;

    pool_market.manager = *new_manager_info.key;

    PoolMarket::pack(pool_market, *pool_market_info.data.borrow_mut())?;

    Ok(())
}

