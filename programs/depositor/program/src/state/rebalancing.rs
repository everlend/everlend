//! Program state definitions

use super::{AccountType, RebalancingStep, TOTAL_REBALANCING_STEP};
use crate::state::RebalancingOperation;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_liquidity_oracle::state::{TokenDistribution, LENDINGS_SIZE};
use everlend_utils::{abs_diff, amount_share, percent_div, EverlendError};
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
    pub money_market_collateral_supply: [u64; LENDINGS_SIZE],

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
        new_token_distribution: TokenDistribution,
        new_liquidity_supply: u64,
    ) -> Result<(), ProgramError> {
        if new_token_distribution.updated_at <= self.token_distribution.updated_at {
            return Err(ProgramError::InvalidArgument);
        }

        // Reset steps
        self.steps = Vec::new();

        // Compute steps
        for i in 0..LENDINGS_SIZE {
            let money_market = new_token_distribution.distribution[i].money_market;

            // If distribution is over
            if money_market == Default::default() {
                break;
            }

            let money_market_liquidity_supply = amount_share(
                self.liquidity_supply,
                self.token_distribution.distribution[i].percent,
            )?;
            let new_money_market_liquidity_supply = amount_share(
                new_liquidity_supply,
                new_token_distribution.distribution[i].percent,
            )?;

            match new_money_market_liquidity_supply.cmp(&money_market_liquidity_supply) {
                Ordering::Greater => {
                    self.add_rebalancing_step(RebalancingStep::new(
                        money_market,
                        RebalancingOperation::Deposit,
                        new_money_market_liquidity_supply - money_market_liquidity_supply,
                        0, // this will be calculated at the deposit stage
                    ));
                }
                Ordering::Less => {
                    let money_market_index = self.get_money_market_index(&money_market);
                    // Compute collateral amount depending on amount percent
                    let collateral_amount = amount_share(
                        self.money_market_collateral_supply[money_market_index],
                        percent_div(
                            new_money_market_liquidity_supply,
                            money_market_liquidity_supply,
                        )?,
                    )?;

                    self.add_rebalancing_step(RebalancingStep::new(
                        money_market,
                        RebalancingOperation::Withdraw,
                        money_market_liquidity_supply - new_money_market_liquidity_supply,
                        collateral_amount,
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
        money_market_program_id: Pubkey,
        operation: RebalancingOperation,
        collateral_amount: Option<u64>,
        slot: Slot,
    ) -> Result<(), ProgramError> {
        let step = self.next_step_mut();
        let collateral_amount = collateral_amount.unwrap_or(step.collateral_amount);

        step.execute(money_market_program_id, operation, slot)?;

        let money_market_index = self.get_money_market_index(&money_market_program_id);

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

    /// Get money market index by program id and token distribution
    pub fn get_money_market_index(&self, money_market_program_id: &Pubkey) -> usize {
        self.token_distribution
            .distribution
            .iter()
            .position(|&d| d.money_market == *money_market_program_id)
            .unwrap()
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
    // 1 + 32 + 32 + 8 + 409 + (4 + 5 * 50) = 736
    const LEN: usize =
        73 + TokenDistribution::LEN + (4 + TOTAL_REBALANCING_STEP * RebalancingStep::LEN);

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
    use everlend_liquidity_oracle::state::{DistributionArray, LiquidityDistribution};

    #[test]
    fn packing() {
        let pk = Pubkey::new_from_array([100; 32]);
        let mut rebalancing: Rebalancing = Default::default();
        rebalancing.init(InitRebalancingParams {
            depositor: pk,
            mint: pk,
        });
        let rebalancing_step = RebalancingStep::new(pk, RebalancingOperation::Deposit, 100, 0);
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

    #[test]
    fn computing() {
        let pk = Pubkey::new_unique();
        let mut rebalancing: Rebalancing = Default::default();
        rebalancing.init(InitRebalancingParams {
            depositor: pk,
            mint: pk,
        });

        let mut token_distribution: TokenDistribution = Default::default();
        let mut distribution = DistributionArray::default();
        distribution[0] = LiquidityDistribution {
            money_market: Pubkey::new_unique(),
            percent: 900_000_000u64,
        };
        distribution[1] = LiquidityDistribution {
            money_market: Pubkey::new_unique(),
            percent: 100_000_000u64,
        };
        token_distribution.update(2, distribution);

        rebalancing
            .compute(token_distribution.clone(), 100_000_000)
            .unwrap();

        assert_eq!(rebalancing.steps.len(), 2);

        distribution[0].percent = 450_000_000u64;
        distribution[1].percent = 250_000_000u64;
        distribution[2] = LiquidityDistribution {
            money_market: Pubkey::new_unique(),
            percent: 300_000_000u64,
        };
        token_distribution.update(3, distribution);

        rebalancing
            .compute(token_distribution, 100_000_000)
            .unwrap();

        assert_eq!(rebalancing.steps[0].amount, 45_000_000);
        assert_eq!(
            rebalancing.steps[0].operation,
            RebalancingOperation::Withdraw
        );
        assert_eq!(rebalancing.steps[1].amount, 15_000_000);
        assert_eq!(
            rebalancing.steps[1].operation,
            RebalancingOperation::Deposit
        );
        assert_eq!(rebalancing.steps[2].amount, 30_000_000);
        assert_eq!(
            rebalancing.steps[2].operation,
            RebalancingOperation::Deposit
        );
    }
}
