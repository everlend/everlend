use everlend_registry::instructions::{UpdateRegistryData, UpdateRegistryMarketsData};
use solana_program::{
    instruction::{AccountMeta, Instruction, InstructionError},
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar,
};
use solana_program_test::*;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, TransactionError};
use std::convert::TryFrom;

use everlend_depositor::{
    find_rebalancing_program_address, find_transit_program_address,
    instruction::DepositorInstruction,
};
use everlend_liquidity_oracle::state::DistributionArray;
use everlend_utils::{
    find_program_address,
    integrations::{self, MoneyMarketPubkeys},
    EverlendError,
};

use anchor_lang::__private::bytemuck;
use borsh::{BorshDeserialize, BorshSerialize};
use jet_proto_math::Number;
use rand::Rng;

use crate::utils::*;

#[tokio::test]
async fn number_bytemuck() {
    #[derive(Debug, Default, BorshSerialize, BorshDeserialize)]
    pub struct MarginPool {
        pub id: [u8; 8],

        pub version: u8,

        pub pool_bump: [u8; 1],

        pub vault: Pubkey,

        pub fee_destination: Pubkey,

        pub deposit_note_mint: Pubkey,

        pub loan_note_mint: Pubkey,

        pub token_mint: Pubkey,

        pub token_price_oracle: Pubkey,

        pub address: Pubkey,

        pub config: MarginPoolConfig,

        pub borrowed_tokens: [u8; 24],

        pub uncollected_fees: [u8; 24],

        pub deposit_tokens: u64,

        pub deposit_notes: u64,

        pub loan_notes: u64,

        pub accrued_until: i64,

        pub end_id: [u8; 8],
    }

    #[derive(Debug, Default, BorshSerialize, BorshDeserialize, Clone, Eq, PartialEq)]
    pub struct MarginPoolConfig {
        pub flags: u64,

        pub utilization_rate_1: u16,

        pub utilization_rate_2: u16,

        pub borrow_rate_0: u16,

        pub borrow_rate_1: u16,

        pub borrow_rate_2: u16,

        pub borrow_rate_3: u16,

        pub management_fee_rate: u16,

        pub reserved: u64,
    }

    let mut arr: [u8; 24] = [0u8; 24];

    for i in 0..23 {
        arr[i] = rand::thread_rng().gen_range(0..255);
    }

    let number_1 = Number::from_bits(arr);
    let number_2 = bytemuck::from_bytes::<Number>(&arr);

    assert_eq!(number_1, *number_2);
}