use borsh::{BorshDeserialize, BorshSerialize};
use everlend_utils::{assert_account_key, next_account, next_signer_account};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

use crate::state::{DistributionPubkeys, Registry, RegistryMarketsConfig};

/// Instruction data
#[derive(BorshDeserialize, BorshSerialize, PartialEq, Debug)]
pub struct UpdateMarketsData {
    ///
    pub money_markets: Option<DistributionPubkeys>,
    ///
    pub collateral_pool_markets: Option<DistributionPubkeys>,
}
/// Instruction context
pub struct UpdateMarketsContext<'a> {
    manager: AccountInfo<'a>,
    registry: AccountInfo<'a>,
}

impl<'a> UpdateMarketsContext<'a> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'a>],
    ) -> Result<UpdateMarketsContext<'a>, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        let registry_info = next_account(account_info_iter, program_id)?;
        let manager_info = next_signer_account(account_info_iter)?;

        Ok(UpdateMarketsContext {
            manager: manager_info.clone(),
            registry: registry_info.clone(),
        })
    }

    /// Process instruction
    pub fn process(&self, _program_id: &Pubkey, data: UpdateMarketsData) -> ProgramResult {
        let r = Registry::unpack(&self.registry.data.borrow())?;
        assert_account_key(&self.manager, &r.manager)?;

        let mut markets = RegistryMarketsConfig::unpack_from_slice(&self.registry.data.borrow())?;
        if let Some(pubkeys) = data.money_markets {
            markets.money_markets = pubkeys;
        }

        if let Some(pubkeys) = data.collateral_pool_markets {
            markets.collateral_pool_markets = pubkeys;
        }

        RegistryMarketsConfig::pack(markets, *self.registry.data.borrow_mut())?;

        Ok(())
    }
}
