use std::slice::Iter;
use borsh::BorshSerialize;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;
use everlend_utils::EverlendError;
use crate::state::{MAX_REWARDS, PRECISION, RewardVault};

#[derive(Debug, BorshDeserialize, BorshSerializem BorshSchema, Default)]
pub struct Mining {
    pub anchor_id: [u8; 8],
    pub reward_pool: Pubkey,
    pub bump: u8,
    pub share: u64,
    pub owner: Pubkey,
    pub indexes: Vec<RewardIndex>
}

impl Mining {
    pub fn initialize(reward_pool: Pubkey, bump: u8, owner: Pubkey) -> Mining {
        Mining {
            anchor_id: Default::default(),
            reward_pool,
            bump,
            share: 0,
            owner,
            indexes: vec![]
        }
    }

    pub fn reward_index_mut(&mut self, reward_mint: Pubkey) -> &mut RewardIndex {
        match self
            .indexes
            .iter()
            .position(|mi| mi.reward_mint == reward_mint)
        {
            Some(i) => &mut self.indeces[i],
            None => {
                self.indeces.push(RewardIndex {
                    reward_mint,
                    ..Default::default()
                });
                self.indexes.last_mut().unwrap()
            }
        }
    }

    pub fn claim(&mut self, reward_mint: Pubkey) {
        let reward_index = self.reward_index_mut(reward_mint);
        reward_index.rewards = 0;
    }

    pub fn refresh_rewards(&mut self, vaults: Iter<RewardVault>) -> ProgramResult {
        let share = self.share;

        for vault in vaults {
            let reward_index = self.reward_index_mut(vault.reward_mint);

            if vault.index_with_precision > reward_index.index_with_precision {
                let rewards = vault.index_with_precision
                    .checked_sub(reward_index.index_with_precision)
                    .ok_or(EverlendError::MathOverflow)?
                    .checked_mul(share as u128)
                    .ok_or(EverlendError::MathOverflow)?
                    .checked_div(PRECISION)
                    .ok_or(EverlendError::MathOverflow)?;

                if rewards > 0 {
                    reward_index.rewards = reward_index.rewards
                        .checked_add(rewards as u64)
                        .ok_or(EverlendError::MathOverflow)?;
                }
            }

            reward_index.index_with_precision = vault.index_with_precision;
        }

        Ok(())
    }
}

impl Sealed for Mining {}
impl Pack for Mining {
    const LEN: usize = 8 + (32 + 1 + 8 + 32 + (4 + RewardIndex::LEN * MAX_REWARDS));

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

#[derive(BorshSerialize, BorshDeserialize, Default, Clone)]
pub struct RewardIndex {
    pub reward_mint: Pubkey,
    pub index_with_precision: u128,
    pub rewards: u64,
}

impl RewardIndex {
    pub const LEN:usice = 32 + 16 + 8;
}