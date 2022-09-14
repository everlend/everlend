use borsh::BorshSerialize;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

#[derive(Debug, BorshDeserialize, BorshSerializem BorshSchema, Default)]
pub struct Config {
    pub authority: Pubkey,
}

impl Config {
    pub fn init(authority: Pubkey) -> Config {
        Config {
            authority
        }
    }
}

impl Sealed for Config {}
impl Pack for Config {
    const LEN: usize = 32;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<Pool>());
            ProgramError::InvalidAccountData
        })
    }
}