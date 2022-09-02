use everlend_utils::{
    assert_rent_exempt, assert_uninitialized, cpi, next_program_account, next_signer_account,
};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::{Sysvar, SysvarId},
};

use crate::state::{InitPoolMarketParams, PoolMarket};

/// Instruction context
pub struct InitPoolMarketContext<'a, 'b> {
    manager: &'a AccountInfo<'b>,
    registry: &'a AccountInfo<'b>,
    pool_market: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitPoolMarketContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<InitPoolMarketContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let pool_market_info = next_program_account(account_info_iter, program_id)?;
        let manager_info = next_signer_account(account_info_iter)?;
        let registry_info = next_program_account(account_info_iter, &everlend_registry::id())?;
        let rent_info = next_program_account(account_info_iter, &Rent::id())?;

        let rent = &Rent::from_account_info(rent_info)?;
        assert_rent_exempt(rent, pool_market_info)?;

        Ok(InitPoolMarketContext {
            manager: manager_info,
            registry: registry_info,
            pool_market: pool_market_info,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        // Get pool market state
        let mut pool_market = PoolMarket::unpack_unchecked(&self.pool_market.data.borrow())?;
        assert_uninitialized(&pool_market)?;

        pool_market.init(InitPoolMarketParams {
            manager: self.manager.key.clone(),
            registry: self.registry.key.clone(),
        });

        PoolMarket::pack(pool_market, *self.pool_market.data.borrow_mut())?;

        Ok(())
    }
}
