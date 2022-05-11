//! Program state definitions

use super::{AccountType, RebalancingStep, TOTAL_REBALANCING_STEP};
use crate::state::RebalancingOperation;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_liquidity_oracle::state::TokenDistribution;
use everlend_registry::state::{DistributionPubkeys, RegistrySettings, TOTAL_DISTRIBUTIONS};
use everlend_utils::{math, EverlendError, PRECISION_SCALER};
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

    /// Distributed liquidity
    pub distributed_liquidity: u64,

    /// Received collateral in each market
    pub received_collateral: [u64; TOTAL_DISTRIBUTIONS],

    /// Current token distribution from liquidity oracle
    pub token_distribution: TokenDistribution,

    /// Rebalancing steps
    pub steps: Vec<RebalancingStep>,

    /// Income refreshed mark to avoid frequent refresh
    pub income_refreshed_at: Slot,
    // Space for future values
    // 20
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
        token_distribution: TokenDistribution,
        distributed_liquidity: u64,
    ) -> Result<(), ProgramError> {
        if token_distribution.updated_at <= self.token_distribution.updated_at {
            return Err(EverlendError::TokenDistributionIsStale.into());
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
            let prev_percent = self.token_distribution.distribution[index];
            let percent = token_distribution.distribution[index];

            let prev_distribution_liquidity =
                math::share(self.distributed_liquidity, prev_percent)?;
            let distribution_liquidity = math::share(distributed_liquidity, percent)?;

            let liquidity_amount =
                math::abs_diff(distribution_liquidity, prev_distribution_liquidity)?;

            match distribution_liquidity.cmp(&prev_distribution_liquidity) {
                // Deposit
                Ordering::Greater => {
                    self.add_step(RebalancingStep::new(
                        index as u8,
                        RebalancingOperation::Deposit,
                        liquidity_amount,
                        None, // Will be calculated at the deposit stage
                    ));
                }
                // Withdraw
                Ordering::Less => {
                    let collateral_percent = PRECISION_SCALER
                        .checked_sub(math::percent_ratio(
                            distribution_liquidity,
                            prev_distribution_liquidity,
                        )? as u128)
                        .ok_or(EverlendError::MathOverflow)?;

                    // Compute collateral amount depending on amount percent
                    let collateral_amount =
                        math::share(self.received_collateral[index], collateral_percent as u64)?;

                    self.add_step(RebalancingStep::new(
                        index as u8,
                        RebalancingOperation::Withdraw,
                        liquidity_amount,
                        Some(collateral_amount),
                    ));
                }
                Ordering::Equal => {}
            }
        }

        // Sort steps
        self.steps
            .sort_by(|a, b| a.operation.partial_cmp(&b.operation).unwrap());

        self.token_distribution = token_distribution;
        self.distributed_liquidity = distributed_liquidity;

        Ok(())
    }

    /// Generate new steps for withdraw all funds and deposit them back in MM pools
    pub fn compute_with_refresh_income(
        &mut self,
        money_market_program_ids: &DistributionPubkeys,
        settings: &RegistrySettings,
        income_refreshed_at: Slot,
        distributed_liquidity: u64,
    ) -> Result<(), ProgramError> {
        if self.income_refreshed_at + settings.refresh_income_interval > income_refreshed_at {
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
            let percent = self.token_distribution.distribution[index];
            // Skip zero withdraws/deposits
            if percent == 0 {
                continue;
            }

            let prev_distribution_liquidity = math::share(self.distributed_liquidity, percent)?;
            let distribution_liquidity = math::share(distributed_liquidity, percent)?;
            let collateral_amount = self.received_collateral[index];

            self.add_step(RebalancingStep::new(
                index as u8,
                RebalancingOperation::Withdraw,
                prev_distribution_liquidity,
                Some(collateral_amount),
            ));

            self.add_step(RebalancingStep::new(
                index as u8,
                RebalancingOperation::Deposit,
                distribution_liquidity,
                None, // Will be calculated at the deposit stage
            ));
        }

        // Sort steps
        self.steps
            .sort_by(|a, b| a.operation.partial_cmp(&b.operation).unwrap());

        self.income_refreshed_at = income_refreshed_at;
        self.distributed_liquidity = distributed_liquidity;

        Ok(())
    }

    /// Cancel current rebalancing by applying executed steps
    pub fn cancel(&mut self) -> Result<(), ProgramError> {
        // Reset steps
        // let mut updated_token_distribution = TokenDistribution::default();
        let TokenDistribution { distribution, .. } = self.token_distribution;
        let mut updated_distribution = distribution;
        let updated_distribution_liquidity = self.distributed_liquidity;

        for step in self.steps.iter().filter(|&s| s.executed_at.is_none()) {
            let money_market_index = usize::from(step.money_market_index);
            let step_distribution_liquidity =
                math::share(self.distributed_liquidity, distribution[money_market_index])?;

            // Compute percent with apply reverted operation
            let percent = match step.operation {
                RebalancingOperation::Deposit => {
                    let updated_step_distribution_liquidity = step_distribution_liquidity
                        .checked_sub(step.liquidity_amount)
                        .ok_or(EverlendError::MathOverflow)?;

                    updated_distribution_liquidity
                        .checked_sub(step.liquidity_amount)
                        .ok_or(EverlendError::MathOverflow)?;

                    math::percent_ratio(
                        step_distribution_liquidity,
                        updated_step_distribution_liquidity,
                    )?
                }
                RebalancingOperation::Withdraw => {
                    let updated_step_distribution_liquidity = step_distribution_liquidity
                        .checked_add(step.liquidity_amount)
                        .ok_or(EverlendError::MathOverflow)?;

                    updated_distribution_liquidity
                        .checked_add(step.liquidity_amount)
                        .ok_or(EverlendError::MathOverflow)?;

                    math::percent_ratio(
                        step_distribution_liquidity,
                        updated_step_distribution_liquidity,
                    )?
                }
            };
            updated_distribution[money_market_index] = percent;
        }
        self.steps.retain(|&s| s.executed_at.is_some());

        msg!("updated_distribution = {:?}", updated_distribution);

        self.token_distribution = self.token_distribution.clone();
        self.token_distribution.distribution = updated_distribution;
        self.distributed_liquidity = updated_distribution_liquidity;

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

        step.execute(slot)?;

        let money_market_index = usize::from(step.money_market_index);
        let collateral_amount =
            received_collateral_amount.unwrap_or_else(|| step.collateral_amount.unwrap());

        // Update collateral ammount
        self.received_collateral[money_market_index] = match operation {
            RebalancingOperation::Deposit => self.received_collateral[money_market_index]
                .checked_add(collateral_amount)
                .ok_or(EverlendError::MathOverflow)?,
            RebalancingOperation::Withdraw => self.received_collateral[money_market_index]
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
    pub fn is_completed(&self) -> bool {
        if self.steps.is_empty() {
            return true;
        }

        self.steps.iter().all(|&step| step.executed_at.is_some())
    }

    /// Compute unused liquidity
    pub fn unused_liquidity(&self) -> Result<u64, ProgramError> {
        let total_percent: u64 = self.token_distribution.distribution.iter().sum();

        math::share_floor(
            self.distributed_liquidity,
            PRECISION_SCALER
                .checked_sub(total_percent as u128)
                .ok_or(EverlendError::MathOverflow)? as u64,
        )
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
    // 1 + 32 + 32 + 8 + (8 * 10) + 89 + (4 + 4 * 28) + 8 + 20 = 386
    const LEN: usize = 73
        + (8 * TOTAL_DISTRIBUTIONS)
        + TokenDistribution::LEN
        + (4 + TOTAL_REBALANCING_STEP * RebalancingStep::LEN)
        + 8
        + 20;

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
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::Rebalancing
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
        let pk = Pubkey::new_unique();
        let mut rebalancing: Rebalancing = Default::default();
        rebalancing.init(InitRebalancingParams {
            depositor: pk,
            mint: pk,
        });

        let mut money_market_program_ids = DistributionPubkeys::default();
        let mut money_market_program_ids = DistributionPubkeys::default();
        money_market_program_ids[0] = pk;
        money_market_program_ids[1] = pk;

        let mut token_distribution: TokenDistribution = Default::default();
        let mut distribution = DistributionArray::default();
        distribution[0] = 900_000_000u64;
        distribution[1] = 100_000_000u64;

        token_distribution.update(2, distribution);

        rebalancing
            .compute(
                &money_market_program_ids,
                token_distribution.clone(),
                100_000_000,
            )
            .unwrap();

        assert_eq!(rebalancing.steps.len(), 2);
        assert_eq!(rebalancing.steps[0].liquidity_amount, 90_000_000);
        assert_eq!(rebalancing.steps[1].liquidity_amount, 10_000_000);

        // TODO: Add new tests after math updates
    }

    #[test]
    fn computing_with_one_zero() {
        let pk = Pubkey::new_unique();
        let mut rebalancing: Rebalancing = Default::default();
        rebalancing.init(InitRebalancingParams {
            depositor: pk,
            mint: pk,
        });

        let mut money_market_program_ids = DistributionPubkeys::default();
        money_market_program_ids[0] = pk;
        money_market_program_ids[1] = pk;

        let mut token_distribution: TokenDistribution = Default::default();
        let mut distribution = DistributionArray::default();
        distribution[0] = 1_000_000_000u64;
        distribution[1] = 0;

        token_distribution.update(2, distribution);

        rebalancing
            .compute(&money_market_program_ids, token_distribution.clone(), 1)
            .unwrap();

        rebalancing
            .execute_step(RebalancingOperation::Deposit, Some(1), 3)
            .unwrap();

        distribution[0] = 0;
        token_distribution.update(4, distribution);
        rebalancing
            .compute(&money_market_program_ids, token_distribution.clone(), 1)
            .unwrap();

        println!("rebalancing = {:#?}", rebalancing);
    }

    #[test]
    fn unused_liquidity() {
        let pk = Pubkey::new_unique();
        let mut rebalancing: Rebalancing = Default::default();
        rebalancing.init(InitRebalancingParams {
            depositor: pk,
            mint: pk,
        });

        let mut money_market_program_ids = DistributionPubkeys::default();
        money_market_program_ids[0] = pk;
        money_market_program_ids[1] = pk;

        let mut token_distribution: TokenDistribution = Default::default();
        let mut distribution = DistributionArray::default();
        distribution[0] = 1000000000;
        distribution[1] = 0;

        token_distribution.update(2, distribution);

        rebalancing
            .compute(
                &money_market_program_ids,
                token_distribution.clone(),
                2100000,
            )
            .unwrap();

        rebalancing
            .execute_step(RebalancingOperation::Deposit, Some(2100000), 3)
            .unwrap();

        distribution[0] = 999999999;
        token_distribution.update(4, distribution);
        rebalancing
            .compute(
                &money_market_program_ids,
                token_distribution.clone(),
                2100000,
            )
            .unwrap();

        println!("rebalancing = {:#?}", rebalancing);
        println!(
            "unused_liquidity = {:#?}",
            rebalancing.unused_liquidity().unwrap()
        );

        distribution[0] = 999999998;
        token_distribution.update(7, distribution);
        rebalancing
            .compute(
                &money_market_program_ids,
                token_distribution.clone(),
                2100000,
            )
            .unwrap();

        println!("rebalancing = {:#?}", rebalancing);
        println!(
            "unused_liquidity = {:#?}",
            rebalancing.unused_liquidity().unwrap()
        );
    }
}
