use crate::{
    find_pool_config_program_address,
    state::{Pool, PoolConfig, PoolMarket, SetPoolConfigParams},
};
use everlend_utils::{
    assert_account_key, assert_owned_by, cpi, next_account, next_optional_account,
    next_program_account, next_signer_account,
};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_program,
    sysvar::{Sysvar, SysvarId},
};

/// Instruction context
pub struct SetPoolConfigContext<'a, 'b> {
    pool_market: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    pool_config: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> SetPoolConfigContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<SetPoolConfigContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let pool_market = next_account(account_info_iter, program_id)?;
        let pool = next_account(account_info_iter, program_id)?;
        let pool_config = next_optional_account(account_info_iter, program_id)?;
        let manager = next_signer_account(account_info_iter)?;
        let rent = next_program_account(account_info_iter, &Rent::id())?;
        let _system_program = next_program_account(account_info_iter, &system_program::id())?;

        Ok(SetPoolConfigContext {
            pool_market,
            manager,
            pool,
            pool_config,
            rent,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey, params: SetPoolConfigParams) -> ProgramResult {
        {
            // Get pool market state
            let pool_market = PoolMarket::unpack(&self.pool_market.data.borrow())?;
            assert_account_key(self.manager, &pool_market.manager)?;

            // Get pool state
            let pool = Pool::unpack(&self.pool.data.borrow())?;
            assert_account_key(self.pool_market, &pool.pool_market)?;
        }

        let (pool_config_pubkey, bump_seed) =
            find_pool_config_program_address(program_id, self.pool.key);
        assert_account_key(self.pool_config, &pool_config_pubkey)?;

        let rent = &Rent::from_account_info(&self.rent)?;

        let mut pool_config = match self.pool_config.lamports() {
            0 => {
                let signers_seeds = &["config".as_bytes(), &self.pool.key.to_bytes(), &[bump_seed]];

                cpi::system::create_account::<PoolConfig>(
                    program_id,
                    self.manager.clone(),
                    self.pool_config.clone(),
                    &[signers_seeds],
                    rent,
                )?;

                PoolConfig::default()
            }
            _ => PoolConfig::unpack(&self.pool_config.data.borrow())?,
        };

        pool_config.set(params);

        PoolConfig::pack(pool_config, *self.pool_config.data.borrow_mut())?;

        Ok(())
    }
}
