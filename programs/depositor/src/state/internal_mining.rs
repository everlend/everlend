use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;

use everlend_utils::AccountVersion;

use crate::state::AccountType;

///
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq)]
pub enum MiningType {
    /// None to disable mining
    None,
    ///
    Larix {
        ///
        #[allow(dead_code)]
        mining_account: Pubkey,
        /// ATA for additional reward
        #[allow(dead_code)]
        additional_reward_token_account: Option<Pubkey>,
    },
    ///
    PortFinance {
        ///
        // TODO move to config
        #[allow(dead_code)]
        staking_program_id: Pubkey,
        ///
        #[allow(dead_code)]
        staking_account: Pubkey,
        ///
        #[allow(dead_code)]
        staking_pool: Pubkey,
        ///
        #[allow(dead_code)]
        obligation: Pubkey,
    },
    ///
    Quarry {
        ///
        #[allow(dead_code)]
        rewarder: Pubkey,
    },
    ///
    Frakt {
        ///
        #[allow(dead_code)]
        deposit_account: Pubkey,
    },
}

/// InternalMining
#[repr(C)]
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct InternalMining {
    /// Account type - Depositor
    pub account_type: AccountType,
    /// Account version
    pub account_version: AccountVersion,
    /// Mining type
    pub mining_type: MiningType,
}

impl InternalMining {
    /// Account actual version
    pub const ACTUAL_VERSION: AccountVersion = AccountVersion::V0;

    /// Initialize a internal mining struct
    pub fn init(&mut self, mining_type: MiningType) {
        self.account_type = AccountType::InternalMining;
        self.mining_type = mining_type;
        self.account_version = Self::ACTUAL_VERSION;
    }
}

impl Sealed for InternalMining {}

impl Pack for InternalMining {
    // 1 + 1 + 1 + 4 * 32
    // Make len as max possible enum size
    const LEN: usize = 131;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src_mut = src;
        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for InternalMining {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::InternalMining
            && self.account_version == Self::ACTUAL_VERSION
    }
}
