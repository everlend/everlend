use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum RewardsInstruction {

    InitializePool,

    AddVault,

    FillVault { amount: u64 },

    InitializeMining,

    DepositMining { amount: u64 },

    WithdrawMining { amount: u64 },

    Claim,
}