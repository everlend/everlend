use crate::state::{AccountType, DeprecatedRewardPool, Mining};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::EverlendError;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;

/// Precision for index calculation
pub const PRECISION: u128 = 10_000_000_000_000_000;
/// Max reward vaults
pub const MAX_REWARDS: usize = 5;

/// Reward pool
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct RewardPool {
    /// Account type - RewardPool
    pub account_type: AccountType,
    /// Rewards root account (ex-Config program account)
    pub rewards_root: Pubkey,
    /// Saved bump for reward pool account
    pub bump: u8,
    /// Liquidity mint
    pub liquidity_mint: Pubkey,
    /// Reward total share
    pub total_share: u64,
    /// A set of all possible rewards that we can get for this pool
    pub vaults: Vec<RewardVault>,
    /// The address responsible for the charge of rewards for users.
    /// It executes deposits on the rewards pools.
    pub deposit_authority: Pubkey,
}

impl RewardPool {
    /// Init reward pool
    pub fn init(params: InitRewardPoolParams) -> RewardPool {
        RewardPool {
            account_type: AccountType::RewardPool,
            rewards_root: params.rewards_root,
            bump: params.bump,
            liquidity_mint: params.liquidity_mint,
            total_share: 0,
            vaults: vec![],
            deposit_authority: params.deposit_authority,
        }
    }

    /// Process add vault
    pub fn add_vault(&mut self, reward: RewardVault) -> ProgramResult {
        self.vaults.push(reward);

        Ok(())
    }

    /// Process fill
    pub fn fill(&mut self, reward_mint: Pubkey, rewards: u64) -> ProgramResult {
        if self.total_share == 0 {
            return Err(EverlendError::RewardsNoDeposits.into());
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

        vault.index_with_precision = vault
            .index_with_precision
            .checked_add(index)
            .ok_or(EverlendError::MathOverflow)?;

        Ok(())
    }

    /// Process deposit
    pub fn deposit(&mut self, mining: &mut Mining, amount: u64) -> ProgramResult {
        mining.refresh_rewards(self.vaults.iter())?;

        self.total_share = self
            .total_share
            .checked_add(amount)
            .ok_or(EverlendError::MathOverflow)?;
        mining.share = mining
            .share
            .checked_add(amount)
            .ok_or(EverlendError::MathOverflow)?;

        Ok(())
    }

    /// Process withdraw
    pub fn withdraw(&mut self, mining: &mut Mining, amount: u64) -> ProgramResult {
        mining.refresh_rewards(self.vaults.iter())?;

        self.total_share = self
            .total_share
            .checked_sub(amount)
            .ok_or(EverlendError::MathOverflow)?;
        mining.share = mining
            .share
            .checked_sub(amount)
            .ok_or(EverlendError::MathOverflow)?;

        Ok(())
    }

    /// Process migrate
    pub fn migrate(deprecated_pool: &DeprecatedRewardPool) -> RewardPool {
        Self {
            account_type: AccountType::RewardPool,
            rewards_root: deprecated_pool.rewards_root,
            bump: deprecated_pool.bump,
            liquidity_mint: deprecated_pool.liquidity_mint,
            total_share: deprecated_pool.total_share,
            vaults: deprecated_pool.vaults.clone(),
            deposit_authority: deprecated_pool.deposit_authority
        }
    }
}

/// Initialize a Reward Pool params
pub struct InitRewardPoolParams {
    /// Rewards Root (ex-Config program account)
    pub rewards_root: Pubkey,
    /// Saved bump for reward pool account
    pub bump: u8,
    /// Liquidity mint
    pub liquidity_mint: Pubkey,
    /// The address responsible for the charge of rewards for users.
    /// It executes deposits on the rewards pools.
    pub deposit_authority: Pubkey,
}

impl Sealed for RewardPool {}
impl Pack for RewardPool {
    /// 1 + 32 + 1 + 32 + 8 + (1 + 32 + 16 + 32) * 5 + 32 = 518
    const LEN: usize = 1 + 32 + 1 + 32 + 8 + RewardVault::LEN * MAX_REWARDS + 32;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<RewardPool, ProgramError> {
        let mut src_mut = src;
        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!("{}", err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for RewardPool {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::RewardPool
    }
}

/// Reward vault
#[derive(Debug, BorshDeserialize, BorshSerialize, BorshSchema, Default, Clone)]
pub struct RewardVault {
    /// Bump of
    pub bump: u8,
    /// Reward mint address
    pub reward_mint: Pubkey,
    /// Index with precision
    pub index_with_precision: u128,
    /// Fee account address
    pub fee_account: Pubkey,
}

impl RewardVault {
    /// 1 + 32 + 16 + 32
    pub const LEN: usize = 81;
}
