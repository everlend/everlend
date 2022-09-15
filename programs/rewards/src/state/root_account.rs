use borsh::{BorshDeserialize, BorshSerialize, BorshSchema};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct RootAccount {
    pub authority: Pubkey,
}

impl RootAccount {
    pub fn init(authority: Pubkey) -> RootAccount {
        RootAccount {
            authority
        }
    }
}

impl Sealed for RootAccount {}
impl Pack for RootAccount {
    const LEN: usize = 32;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<RootAccount>());
            ProgramError::InvalidAccountData
        })
    }
}