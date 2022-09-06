use everlend_utils::{assert_rent_exempt, assert_uninitialized, AccountLoader};
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
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> InitPoolMarketContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<InitPoolMarketContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let pool_market = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let manager = AccountLoader::next_unchecked(account_info_iter)?;
        let registry = AccountLoader::next_with_owner(account_info_iter, &everlend_registry::id())?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;

        Ok(InitPoolMarketContext {
            manager,
            registry,
            pool_market,
            rent,
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey) -> ProgramResult {
        {
            // Get pool market state
            let pool_market = PoolMarket::unpack_unchecked(&self.pool_market.data.borrow())?;
            assert_uninitialized(&pool_market)?;
        }

        {
            let rent = &Rent::from_account_info(self.rent)?;
            assert_rent_exempt(rent, self.pool_market)?;
        }

        let pool_market = PoolMarket::init(InitPoolMarketParams {
            manager: *self.manager.key,
            registry: *self.registry.key,
        });

        PoolMarket::pack(pool_market, *self.pool_market.data.borrow_mut())?;

        Ok(())
    }
}
