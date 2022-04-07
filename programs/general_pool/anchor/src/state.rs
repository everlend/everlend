use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_pack::Pack;
use std::ops::Deref;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoolMarket(everlend_general_pool::state::PoolMarket);

impl PoolMarket {
    pub const LEN: usize = everlend_general_pool::state::PoolMarket::LEN;

    /// Computes the minimum rent exempt balance of a [PoolMarket].
    pub fn minimum_rent_exempt_balance() -> Result<u64> {
        Ok(Rent::get()?.minimum_balance(Self::LEN))
    }
}

impl Owner for PoolMarket {
    fn owner() -> Pubkey {
        crate::ID
    }
}

impl Deref for PoolMarket {
    type Target = everlend_general_pool::state::PoolMarket;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl anchor_lang::AccountSerialize for PoolMarket {
    fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> Result<()> {
        // no-op
        Ok(())
    }
}

impl anchor_lang::AccountDeserialize for PoolMarket {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        PoolMarket::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        Ok(everlend_general_pool::state::PoolMarket::unpack(buf).map(PoolMarket)?)
    }
}
