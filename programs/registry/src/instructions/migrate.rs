use crate::state::{
    DeprecatedRegistry, DeprecatedRegistryMarkets, MoneyMarkets, Registry, RegistryMarkets,
};
use everlend_utils::{cpi, AccountLoader, assert_account_key};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_program;
use solana_program::sysvar::{Sysvar, SysvarId};

/// Instruction context
pub struct MigrateContext<'a, 'b> {
    registry: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> MigrateContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<MigrateContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let registry = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let manager = AccountLoader::next_signer(account_info_iter)?;
        let _system = AccountLoader::next_with_key(account_info_iter, &system_program::id())?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;

        Ok(MigrateContext {
            registry,
            manager,
            rent,
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey, money_markets: MoneyMarkets) -> ProgramResult {
        let rent = &Rent::from_account_info(self.rent)?;

        let deprecated_registry =
            DeprecatedRegistry::unpack_from_slice(&self.registry.data.borrow())?;
        let deprecated_registry_markets =
            DeprecatedRegistryMarkets::unpack_from_slice(&self.registry.data.borrow())?;

        assert_account_key(self.manager, &deprecated_registry.manager)?;

        let registry = Registry::migrate(&deprecated_registry);
        let registry_markets =
            RegistryMarkets::migrate(&deprecated_registry_markets, money_markets);

        cpi::system::realloc_with_rent(self.registry, self.manager, rent, Registry::LEN)?;

        Registry::pack(registry, *self.registry.data.borrow_mut())?;
        RegistryMarkets::pack_into_slice(&registry_markets, *self.registry.data.borrow_mut());

        Ok(())
    }
}
