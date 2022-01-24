//! Program state definitions

use super::{AccountType, RebalancingStep, TOTAL_REBALANCING_STEP};
use crate::state::RebalancingOperation;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_liquidity_oracle::state::TokenDistribution;
use everlend_registry::state::{RegistryConfig, TOTAL_DISTRIBUTIONS};
use everlend_utils::{abs_diff, amount_share, percent_div, EverlendError, PRECISION_SCALER};
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

    /// Total liquidity supply
    pub liquidity_supply: u64,

    /// Collateral supply in each market
    pub money_market_collateral_supply: [u64; TOTAL_DISTRIBUTIONS],

    /// Latest token distribution from liquidity oracle
    pub token_distribution: TokenDistribution,

    /// Rebalancing steps
    pub steps: Vec<RebalancingStep>,
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
        registry_config: &RegistryConfig,
        new_token_distribution: TokenDistribution,
        new_liquidity_supply: u64,
    ) -> Result<(), ProgramError> {
        if new_token_distribution.updated_at <= self.token_distribution.updated_at {
            return Err(ProgramError::InvalidArgument);
        }

        // Reset steps
        self.steps = Vec::new();

        // Compute steps
        for i in 0..TOTAL_DISTRIBUTIONS {
            let money_market_program_id = registry_config.money_market_program_ids[i];

            // If distribution is over
            if money_market_program_id == Default::default() {
                break;
            }

            let percent = self.token_distribution.distribution[i];
            let new_percent = new_token_distribution.distribution[i];

            let money_market_liquidity_supply = amount_share(self.liquidity_supply, percent)?;
            let new_money_market_liquidity_supply =
                amount_share(new_liquidity_supply, new_percent)?;
            let amount = abs_diff(
                new_money_market_liquidity_supply,
                money_market_liquidity_supply,
            )?;

            match new_money_market_liquidity_supply.cmp(&money_market_liquidity_supply) {
                Ordering::Greater => {
                    self.add_rebalancing_step(RebalancingStep::new(
                        i as u8,
                        RebalancingOperation::Deposit,
                        amount,
                        None, // this will be calculated at the deposit stage
                    ));
                }
                Ordering::Less => {
                    let collateral_percent = PRECISION_SCALER
                        .checked_sub(percent_div(
                            new_money_market_liquidity_supply,
                            money_market_liquidity_supply,
                        )? as u128)
                        .ok_or(EverlendError::MathOverflow)?;

                    // Compute collateral amount depending on amount percent
                    let collateral_amount = amount_share(
                        self.money_market_collateral_supply[i],
                        collateral_percent as u64,
                    )?;

                    self.add_rebalancing_step(RebalancingStep::new(
                        i as u8,
                        RebalancingOperation::Withdraw,
                        amount,
                        Some(collateral_amount),
                    ));
                }
                Ordering::Equal => {}
            }
        }

        // Sort steps
        self.steps
            .sort_by(|a, b| a.operation.partial_cmp(&b.operation).unwrap());

        self.token_distribution = new_token_distribution;
        self.liquidity_supply = new_liquidity_supply;

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
        self.money_market_collateral_supply[money_market_index] = match operation {
            RebalancingOperation::Deposit => self.money_market_collateral_supply
                [money_market_index]
                .checked_add(collateral_amount)
                .ok_or(EverlendError::MathOverflow)?,
            RebalancingOperation::Withdraw => self.money_market_collateral_supply
                [money_market_index]
                .checked_sub(collateral_amount)
                .ok_or(EverlendError::MathOverflow)?,
        };

        Ok(())
    }

    /// Add rebalancing step
    pub fn add_rebalancing_step(&mut self, rebalancing_step: RebalancingStep) {
        self.steps.push(rebalancing_step);
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
    // 1 + 32 + 32 + 8 + (8 * 10) + 89 + (4 + 5 * 28) = 386
    const LEN: usize = 73
        + (8 * TOTAL_DISTRIBUTIONS)
        + TokenDistribution::LEN
        + (4 + TOTAL_REBALANCING_STEP * RebalancingStep::LEN);

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

    #[test]
    fn packing() {
        let pk = Pubkey::new_from_array([100; 32]);
        let mut rebalancing: Rebalancing = Default::default();
        rebalancing.init(InitRebalancingParams {
            depositor: pk,
            mint: pk,
        });
        let rebalancing_step = RebalancingStep::new(0, RebalancingOperation::Deposit, 100, None);
        rebalancing.add_rebalancing_step(rebalancing_step);
        rebalancing.add_rebalancing_step(rebalancing_step);

        let rebalancing_clone = rebalancing.clone();

        let mut expected: [u8; Rebalancing::LEN] = [0; Rebalancing::LEN];
        Rebalancing::pack(rebalancing, &mut expected).unwrap();

        assert_eq!(Rebalancing::LEN, expected.len());
        assert_eq!(
            Rebalancing::unpack_unchecked(&expected).unwrap(),
            rebalancing_clone
        );
    }
}
