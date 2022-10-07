//! Program state definitions

use super::{AccountType, RebalancingStep, TOTAL_REBALANCING_STEP};
use crate::state::RebalancingOperation;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_liquidity_oracle::state::{Distribution, DistributionArray, TokenOracle};
use everlend_registry::state::{DistributionPubkeys, TOTAL_DISTRIBUTIONS};
use everlend_utils::{math, EverlendError};
use solana_program::{
    clock::Slot,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::cmp::Ordering;

/// Rebalancing
#[repr(C)]
#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct Rebalancing {
    /// Account type - Rebalancing
    pub account_type: AccountType,

    /// Depositor
    pub depositor: Pubkey,

    /// Mint
    pub mint: Pubkey,

    /// Amount to distribute
    pub amount_to_distribute: u64,

    /// Distributed liquidity into MMs
    pub distributed_liquidity: DistributionArray,

    /// Received collateral in each market
    pub received_collateral: DistributionArray,

    /// Current token distribution from liquidity oracle
    pub liquidity_distribution: Distribution,

    /// Rebalancing steps
    pub steps: Vec<RebalancingStep>,

    /// Income refreshed mark to avoid frequent refresh
    pub income_refreshed_at: Slot,
}

impl Rebalancing {
    /// Initialize a rebalancing
    pub fn init(&mut self, params: InitRebalancingParams) {
        self.account_type = AccountType::Rebalancing;
        self.depositor = params.depositor;
        self.mint = params.mint;
    }

    /// Generate new steps from new and latest distribuition arrays
    pub fn compute(
        &mut self,
        money_market_program_ids: &DistributionPubkeys,
        token_oracle: TokenOracle,
        amount_to_distribute: u64,
        current_slot: Slot,
    ) -> Result<(), ProgramError> {
        if token_oracle.liquidity_distribution.updated_at <= self.liquidity_distribution.updated_at
        {
            return Err(EverlendError::LiquidityDistributionStale.into());
        }

        if token_oracle.reserve_rates.updated_at != current_slot {
            return Err(EverlendError::ReserveRatesStale.into());
        }

        // Reset steps
        self.steps = Vec::new();

        // Compute steps
        for (index, _) in money_market_program_ids
            .iter()
            .enumerate()
            // Keeping index order
            .filter(|&id| *id.1 != Default::default())
        {
            // Spread percents
            let percent = token_oracle.liquidity_distribution.values[index];
            let prev_amount = self.distributed_liquidity[index];
            let new_amount = math::share_floor(amount_to_distribute, percent)?;

            let amount = math::abs_diff(new_amount, prev_amount)?;

            let distributed_liquidity = match new_amount.cmp(&prev_amount) {
                // Deposit
                Ordering::Greater => {
                    // Сheck collateral leak (only if it's set for market)
                    let collateral_percent = token_oracle.reserve_rates.values[index];
                    let expected_collateral = math::share_floor(amount, collateral_percent)?;
                    if collateral_percent > 0 && expected_collateral == 0 {
                        // Do nothing and preserve old amount
                        prev_amount
                    } else {
                        // Deposit new liquidity
                        self.add_step(RebalancingStep::new(
                            index as u8,
                            RebalancingOperation::Deposit,
                            amount,
                            None, // Will be calculated at the deposit stage
                        ));

                        new_amount
                    }
                }

                // Withdraw
                Ordering::Less => {
                    let collateral_amount =
                        math::percent_ratio(amount, prev_amount, self.received_collateral[index])?;

                    self.add_step(RebalancingStep::new(
                        index as u8,
                        RebalancingOperation::Withdraw,
                        amount,
                        Some(collateral_amount),
                    ));

                    new_amount
                }
                Ordering::Equal => new_amount,
            };

            self.distributed_liquidity[index] = distributed_liquidity;
        }

        // Sort steps
        self.steps
            .sort_by(|a, b| a.operation.partial_cmp(&b.operation).unwrap());

        self.liquidity_distribution = token_oracle.liquidity_distribution;
        self.amount_to_distribute = amount_to_distribute;

        Ok(())
    }

    /// Generate new steps for withdraw all funds and deposit them back in MM pools
    pub fn compute_with_refresh_income(
        &mut self,
        money_market_program_ids: &DistributionPubkeys,
        refresh_income_interval: u64,
        income_refreshed_at: Slot,
        amount_to_distribute: u64,
    ) -> Result<(), ProgramError> {
        if self.income_refreshed_at + refresh_income_interval > income_refreshed_at {
            return Err(EverlendError::IncomeRefreshed.into());
        }

        // Reset steps
        self.steps = Vec::new();

        // Compute steps
        for (index, _) in money_market_program_ids
            .iter()
            .enumerate()
            // Keeping index order
            .filter(|&id| *id.1 != Default::default())
        {
            // Spread percents
            let percent = self.liquidity_distribution.values[index];
            let prev_amount = self.distributed_liquidity[index];
            let new_amount = math::share_floor(amount_to_distribute, percent)?;

            if prev_amount == 0 && new_amount == 0 {
                continue;
            }

            if prev_amount.gt(&0) {
                let collateral_amount = self.received_collateral[index];

                self.add_step(RebalancingStep::new(
                    index as u8,
                    RebalancingOperation::RefreshWithdraw,
                    prev_amount,
                    Some(collateral_amount),
                ));
            }

            //TODO should be necessary for batch processing?
            if new_amount.gt(&0) {
                self.add_step(RebalancingStep::new(
                    index as u8,
                    RebalancingOperation::RefreshDeposit,
                    new_amount,
                    None, // Will be calculated at the deposit stage
                ));
            }

            self.distributed_liquidity[index] = new_amount;
        }

        self.income_refreshed_at = income_refreshed_at;
        self.amount_to_distribute = amount_to_distribute;

        Ok(())
    }

    /// Set current rebalancing
    pub fn set(
        &mut self,
        amount_to_distribute: u64,
        distributed_liquidity: DistributionArray,
        distribution_array: DistributionArray,
    ) -> Result<(), ProgramError> {
        self.steps.retain(|&s| s.executed_at.is_some());
        self.amount_to_distribute = amount_to_distribute;
        self.distributed_liquidity = distributed_liquidity;
        self.liquidity_distribution.values = distribution_array;

        Ok(())
    }

    /// Get next unexecuted rebalancing step
    pub fn next_step(&self) -> &RebalancingStep {
        self.steps
            .iter()
            .find(|&&step| step.executed_at.is_none())
            .unwrap()
    }

    /// Get mutable next unexecuted rebalancing step
    pub fn next_step_mut(&mut self) -> &mut RebalancingStep {
        self.steps
            .iter_mut()
            .find(|&&mut step| step.executed_at.is_none())
            .unwrap()
    }

    /// Get next unexecuted refresh rebalancing steps
    pub fn next_refresh_steps(&self) -> Result<(RebalancingStep, RebalancingStep), ProgramError> {
        let index = self
            .steps
            .iter()
            .position(|&step| step.executed_at.is_none())
            .unwrap();
        // If index is last element
        if index == (self.steps.len() - 1) {
            return Err(EverlendError::InvalidRebalancingOperation.into());
        }

        Ok((self.steps[index], self.steps[index + 1]))
    }

    /// Execute next unexecuted rebalancing step
    pub fn execute_step(
        &mut self,
        operation: RebalancingOperation,
        received_collateral_amount: Option<u64>,
        slot: Slot,
    ) -> Result<(), ProgramError> {
        let step = self.next_step_mut();
        if step.operation != operation {
            return Err(EverlendError::InvalidRebalancingOperation.into());
        }

        step.set_executed_at(slot);

        let money_market_index = usize::from(step.money_market_index);
        let collateral_amount =
            received_collateral_amount.unwrap_or_else(|| step.collateral_amount.unwrap());

        // Update collateral ammount
        self.received_collateral[money_market_index] = match operation {
            RebalancingOperation::Deposit | RebalancingOperation::RefreshDeposit => self
                .received_collateral[money_market_index]
                .checked_add(collateral_amount)
                .ok_or(EverlendError::MathOverflow)?,
            RebalancingOperation::Withdraw | RebalancingOperation::RefreshWithdraw => self
                .received_collateral[money_market_index]
                .checked_sub(collateral_amount)
                .ok_or(EverlendError::MathOverflow)?,
        };

        Ok(())
    }

    /// Add rebalancing step
    pub fn add_step(&mut self, rebalancing_step: RebalancingStep) {
        self.steps.push(rebalancing_step);
    }

    /// Check all steps are executed
    pub fn total_distributed_liquidity(&self) -> u64 {
        self.distributed_liquidity.iter().sum()
    }

    /// Check all steps are executed
    pub fn is_completed(&self) -> bool {
        if self.steps.is_empty() {
            return true;
        }

        self.steps.iter().all(|&step| step.executed_at.is_some())
    }
}

/// Initialize a Rebalancing params
pub struct InitRebalancingParams {
    /// Depositor
    pub depositor: Pubkey,
    /// Mint
    pub mint: Pubkey,
}

impl Sealed for Rebalancing {}
impl Pack for Rebalancing {
    const LEN: usize = 1
        + 32
        + 32
        + 8
        + 8
        + (8 * TOTAL_DISTRIBUTIONS)
        + Distribution::LEN
        + (4 + TOTAL_REBALANCING_STEP * RebalancingStep::LEN)
        + 8;

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

impl IsInitialized for Rebalancing {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::Rebalancing
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::state::RebalancingOperation;
    use everlend_liquidity_oracle::state::DistributionArray;

    #[test]
    fn packing() {
        let pk = Pubkey::new_from_array([100; 32]);
        let mut rebalancing: Rebalancing = Default::default();
        rebalancing.init(InitRebalancingParams {
            depositor: pk,
            mint: pk,
        });
        let rebalancing_step = RebalancingStep::new(0, RebalancingOperation::Deposit, 100, None);
        rebalancing.add_step(rebalancing_step);
        rebalancing.add_step(rebalancing_step);

        let rebalancing_clone = rebalancing.clone();

        let mut expected: [u8; Rebalancing::LEN] = [0; Rebalancing::LEN];
        Rebalancing::pack(rebalancing, &mut expected).unwrap();

        assert_eq!(Rebalancing::LEN, expected.len());
        assert_eq!(
            Rebalancing::unpack_unchecked(&expected).unwrap(),
            rebalancing_clone
        );
    }

    #[test]
    fn computing() {
        let current_slot = 1;
        let pk = Pubkey::new_unique();
        let mut rebalancing: Rebalancing = Default::default();
        rebalancing.init(InitRebalancingParams {
            depositor: pk,
            mint: pk,
        });

        let mut money_market_program_ids = DistributionPubkeys::default();
        money_market_program_ids[0] = pk;
        money_market_program_ids[1] = pk;

        let mut oracle: TokenOracle = Default::default();
        let mut distribution = DistributionArray::default();
        distribution[0] = 900_000_000u64;
        distribution[1] = 100_000_000u64;

        oracle.reserve_rates.updated_at = current_slot;
        oracle
            .update_liquidity_distribution(2, distribution)
            .unwrap();

        rebalancing
            .compute(
                &money_market_program_ids,
                oracle.clone(),
                100_000_000,
                current_slot,
            )
            .unwrap();

        assert_eq!(rebalancing.steps.len(), 2);
        assert_eq!(rebalancing.steps[0].liquidity_amount, 90_000_000);
        assert_eq!(rebalancing.steps[1].liquidity_amount, 10_000_000);

        // TODO: Add new tests after math updates
    }

    #[test]
    fn computing_with_one_zero() {
        let current_slot = 1;
        let pk = Pubkey::new_unique();
        let mut rebalancing: Rebalancing = Default::default();
        rebalancing.init(InitRebalancingParams {
            depositor: pk,
            mint: pk,
        });

        let mut money_market_program_ids = DistributionPubkeys::default();
        money_market_program_ids[0] = pk;
        money_market_program_ids[1] = pk;

        let mut token_oracle: TokenOracle = Default::default();
        let mut distribution = DistributionArray::default();
        distribution[0] = 1_000_000_000u64;
        distribution[1] = 0;

        token_oracle.reserve_rates.updated_at = current_slot;
        token_oracle
            .update_liquidity_distribution(2, distribution)
            .unwrap();

        rebalancing
            .compute(
                &money_market_program_ids,
                token_oracle.clone(),
                1,
                current_slot,
            )
            .unwrap();

        rebalancing
            .execute_step(RebalancingOperation::Deposit, Some(1), 3)
            .unwrap();

        distribution[0] = 0;
        token_oracle
            .update_liquidity_distribution(4, distribution)
            .unwrap();
        rebalancing
            .compute(
                &money_market_program_ids,
                token_oracle.clone(),
                1,
                current_slot,
            )
            .unwrap();

        println!("rebalancing = {:#?}", rebalancing);
    }
}
