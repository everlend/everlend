mod accounts;
mod instructions;
mod state;

pub use accounts::*;
pub use instructions::*;
pub use state::*;

use anchor_lang::prelude::*;

declare_id!("GenUMNGcWca1GiPLfg89698Gfys1dzk9BAGsyb9aEL2u");

/// The GeneralPool program.
#[derive(Clone)]
pub struct GeneralPool;

impl anchor_lang::AccountDeserialize for GeneralPool {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        GeneralPool::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(_buf: &mut &[u8]) -> Result<Self> {
        Ok(GeneralPool)
    }
}

impl anchor_lang::Id for GeneralPool {
    fn id() -> Pubkey {
        ID
    }
}
