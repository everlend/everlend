use borsh::{BorshDeserialize, BorshSerialize};
use everlend_utils::{assert_account_key, AccountLoader};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey, slot_history::Slot,
};

use crate::state::Registry;

/// Instruction data
#[derive(BorshDeserialize, BorshSerialize, Clone, PartialEq, Debug)]
pub struct UpdateRegistryData {
    ///
    pub general_pool_market: Option<Pubkey>,
    ///
    pub income_pool_market: Option<Pubkey>,
    ///
    pub liquidity_oracle: Option<Pubkey>,
    ///
    pub refresh_income_interval: Option<Slot>,
}

/// Instruction context
pub struct UpdateRegistryContext<'a, 'b> {
    registry: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
}

impl<'a, 'b> UpdateRegistryContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<UpdateRegistryContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();
        let registry = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let manager = AccountLoader::next_signer(account_info_iter)?;

        Ok(UpdateRegistryContext { registry, manager })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey, data: UpdateRegistryData) -> ProgramResult {
        let mut r = Registry::unpack(&self.registry.data.borrow())?;
        assert_account_key(self.manager, &r.manager)?;

        if let Some(general_pool_market) = data.general_pool_market {
            r.general_pool_market = general_pool_market;
        }

        if let Some(income_pool_market) = data.income_pool_market {
            r.income_pool_market = income_pool_market;
        }

        if let Some(liquidity_oracle) = data.liquidity_oracle {
            r.liquidity_oracle = liquidity_oracle;
        }

        if let Some(refresh_income_interval) = data.refresh_income_interval {
            r.refresh_income_interval = refresh_income_interval;
        }

        Registry::pack(r, *self.registry.data.borrow_mut())?;

        Ok(())
    }
}
