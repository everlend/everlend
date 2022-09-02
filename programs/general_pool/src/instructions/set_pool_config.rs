// /// Setup pool limits and more settings
// pub fn set_pool_config(
//     program_id: &Pubkey,
//     accounts: &[AccountInfo],
//     params: SetPoolConfigParams,
// ) -> ProgramResult {
//     let account_info_iter = &mut accounts.iter();
//     let pool_market_info = next_account_info(account_info_iter)?;
//     let pool_info = next_account_info(account_info_iter)?;
//     let pool_config_info = next_account_info(account_info_iter)?;
//     let manager_info = next_account_info(account_info_iter)?;
//     let rent_info = next_account_info(account_info_iter)?;
//     let rent = &Rent::from_account_info(rent_info)?;
//     let _system_program_info = next_account_info(account_info_iter)?;

//     assert_signer(manager_info)?;

//     // Check programs
//     assert_owned_by(pool_market_info, program_id)?;
//     assert_owned_by(pool_info, program_id)?;

//     // Get pool market state
//     let pool_market = PoolMarket::unpack(&pool_market_info.data.borrow())?;
//     assert_account_key(manager_info, &pool_market.manager)?;

//     // Get pool state
//     let pool = Pool::unpack(&pool_info.data.borrow())?;
//     assert_account_key(pool_market_info, &pool.pool_market)?;

//     let (pool_config_pubkey, bump_seed) =
//         find_pool_config_program_address(program_id, pool_info.key);
//     assert_account_key(pool_config_info, &pool_config_pubkey)?;

//     let mut pool_config = match pool_config_info.lamports() {
//         0 => {
//             let signers_seeds = &["config".as_bytes(), &pool_info.key.to_bytes(), &[bump_seed]];

//             cpi::system::create_account::<PoolConfig>(
//                 program_id,
//                 manager_info.clone(),
//                 pool_config_info.clone(),
//                 &[signers_seeds],
//                 rent,
//             )?;

//             PoolConfig::default()
//         }
//         _ => {
//             assert_owned_by(pool_config_info, program_id)?;
//             PoolConfig::unpack(&pool_config_info.data.borrow())?
//         }
//     };

//     pool_config.set(params);

//     PoolConfig::pack(pool_config, *pool_config_info.data.borrow_mut())?;

//     Ok(())
// }
