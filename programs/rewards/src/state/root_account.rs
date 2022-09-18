use borsh::{BorshDeserialize, BorshSerialize, BorshSchema};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;

/// Root account
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct RootAccount {
    /// Anchor id(For Anchor legacy contract compatibility)
    pub anchor_id: [u8; 8],
    /// Authority address
    pub authority: Pubkey,
}

impl Sealed for RootAccount {}
impl Pack for RootAccount {
    const LEN: usize = 8 + (32);

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src_mut = src;
        Self::deserialize(&mut src_mut).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<RootAccount>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for RootAccount {
    fn is_initialized(&self) -> bool {
        self.authority != Pubkey::default()
    }
}