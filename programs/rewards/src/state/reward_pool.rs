use borsh::{BorshSerialize, BorshDeserialize, BorshSchema};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;
use everlend_utils::EverlendError;
use crate::state::Mining;

pub const PRECISION: u128 = 1_000_000_000_000_000_0;
pub const MAX_REWARDS: usize = 5;

/// Reward pool
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct RewardPool {

    pub root_account: Pubkey,
    pub bump: u8,

    pub liquidity_mint: Pubkey,

    pub total_share: u64,

    /// A set of all possible rewards that we can get for this pool
    pub vaults: Vec<RewardVault>,

    /// The address responsible for the charge of rewards for users.
    /// It executes deposits on the rewards pools.
    pub deposit_authority: Pubkey,
}

impl RewardPool {
    pub fn init(
        params: InitRewardPoolParams
    ) -> RewardPool {
        RewardPool {
            root_account: params.root_account,
            bump: params.bump,
            liquidity_mint: params.liquidity_mint,
            total_share: 0,
            vaults: vec![],
            deposit_authority: params.deposit_authority
        }
    }

    pub fn add_vault(
        &mut self,
        reward: RewardVault,
    ) -> ProgramResult {
        self.vaults.push(reward);

        Ok(())
    }

    pub fn fill(
        &mut self,
        reward_mint: Pubkey,
        rewards: u64
    ) -> ProgramResult {
        if self.total_share == 0 {
            return Err(EverlendError::RewardsNoDeposits.into())
        }

        let vault = self
            .vaults
            .iter_mut()
            .find(|v| v.reward_mint == reward_mint)
            .ok_or(EverlendError::RewardsInvalidVault)?;

        let index = PRECISION
            .checked_mul(rewards as u128)
            .ok_or(EverlendError::MathOverflow)?
            .checked_div(self.total_share as u128)
            .ok_or(EverlendError::MathOverflow)?;

        vault.index_with_precision = vault.index_with_precision
            .checked_add(index)
            .ok_or(EverlendError::MathOverflow)?;

        Ok(())
    }

    pub fn deposit(
        &mut self,
        mining: &mut Mining,
        amount: u64,
    ) -> ProgramResult {
        mining.refresh_rewards(self.vaults.iter())?;

        self.total_share = self.total_share.checked_add(amount).ok_or(EverlendError::MathOverflow)?;
        mining.share = mining.share.checked_add(amount).ok_or(EverlendError::MathOverflow)?;

        Ok(())
    }

    pub fn withdraw(
        &mut self,
        mining: &mut Mining,
        amount: u64,
    ) -> ProgramResult {
        mining.refresh_rewards(self.vaults.iter())?;

        self.total_share = self.total_share.checked_sub(amount).ok_or(EverlendError::MathOverflow)?;
        mining.share = mining.share.checked_sub(amount).ok_or(EverlendError::MathOverflow)?;

        Ok(())
    }
}

/// Initialize a Reward Pool params
pub struct InitRewardPoolParams {
    /// Pool market
    pub root_account: Pubkey,
    ///
    pub bump: u8,
    ///
    pub liquidity_mint: Pubkey,
    ///
    pub deposit_authority: Pubkey,
}

impl Sealed for RewardPool {}
impl Pack for RewardPool {
    const LEN: usize = 8 + (32 + 1 + 32 + 8 + (4 + RewardVault::LEN * MAX_REWARDS) + 32);

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<RewardPool, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<RewardPool>());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for RewardPool {
    fn is_initialized(&self) -> bool {
        self.root_account != Pubkey::default()
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct RewardVault {
    pub bump: u8,
    pub reward_mint: Pubkey,
    pub index_with_precision: u128,
    pub fee_account: Pubkey,
}

impl RewardVault {
    pub const LEN: usize = 1 + 32 + 16 + 32;
}

