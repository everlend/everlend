use borsh::{BorshDeserialize, BorshSerialize};
use everlend_utils::{assert_account_key, next_account, next_signer_account};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

use crate::state::{DistributionPubkeys, Registry, RegistryMarkets};

/// Instruction data
#[derive(BorshDeserialize, BorshSerialize, Clone, PartialEq, Debug)]

pub struct UpdateRegistryMarketsData {
    ///
    pub money_markets: Option<DistributionPubkeys>,
    ///
    pub collateral_pool_markets: Option<DistributionPubkeys>,
}
/// Instruction context
pub struct UpdateRegistryMarketsContext<'a, 'b> {
    registry: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
}

impl<'a, 'b> UpdateRegistryMarketsContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<UpdateRegistryMarketsContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        let registry = next_account(account_info_iter, program_id)?;
        let manager = next_signer_account(account_info_iter)?;

        Ok(UpdateRegistryMarketsContext { registry, manager })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey, data: UpdateRegistryMarketsData) -> ProgramResult {
        {
            let r = Registry::unpack(&self.registry.data.borrow())?;
            assert_account_key(&self.manager, &r.manager)?;
        }

        let mut markets = RegistryMarkets::unpack_from_slice(&self.registry.data.borrow())?;
        if let Some(pubkeys) = data.money_markets {
            markets.money_markets = pubkeys;
        }

        if let Some(pubkeys) = data.collateral_pool_markets {
            markets.collateral_pool_markets = pubkeys;
        }

        RegistryMarkets::pack_into_slice(&markets, *self.registry.data.borrow_mut());

        Ok(())
    }
}
