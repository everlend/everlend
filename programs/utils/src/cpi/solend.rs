use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::{
    account_info::AccountInfo,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};

/// `global:deposit_liquidity` anchor program instruction
const CLAIM_INSTRUCTION: [u8; 8] = [62, 198, 214, 193, 213, 159, 108, 210];

#[derive(Clone, Copy, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ClaimData {
    bump: u8,
    index: u64,
    amount: u64,
    proof: [u8; 32],
}

pub fn refresh_reserve<'a>(
    program_id: &Pubkey,
    reserve: AccountInfo<'a>,
    reserve_liquidity_pyth_oracle: AccountInfo<'a>,
    reserve_liquidity_switchboard_oracle: AccountInfo<'a>,
    clock: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let ix = solend_program::instruction::refresh_reserve(
        *program_id,
        *reserve.key,
        *reserve_liquidity_pyth_oracle.key,
        *reserve_liquidity_switchboard_oracle.key,
    );

    invoke(
        &ix,
        &[
            reserve,
            reserve_liquidity_pyth_oracle,
            reserve_liquidity_switchboard_oracle,
            clock,
        ],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    program_id: &Pubkey,
    source_liquidity: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    reserve_collateral_mint: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = solend_program::instruction::deposit_reserve_liquidity(
        *program_id,
        amount,
        *source_liquidity.key,
        *destination_collateral.key,
        *reserve.key,
        *reserve_liquidity_supply.key,
        *reserve_collateral_mint.key,
        *lending_market.key,
        *authority.key,
    );

    invoke_signed(
        &ix,
        &[
            source_liquidity,
            destination_collateral,
            reserve,
            reserve_liquidity_supply,
            reserve_collateral_mint,
            lending_market,
            lending_market_authority,
            authority,
            clock,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn redeem<'a>(
    program_id: &Pubkey,
    source_collateral: AccountInfo<'a>,
    destination_liquidity: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    reserve_collateral_mint: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = solend_program::instruction::redeem_reserve_collateral(
        *program_id,
        amount,
        *source_collateral.key,
        *destination_liquidity.key,
        *reserve.key,
        *reserve_collateral_mint.key,
        *reserve_liquidity_supply.key,
        *lending_market.key,
        *authority.key,
    );

    invoke_signed(
        &ix,
        &[
            source_collateral,
            reserve,
            reserve_collateral_mint,
            reserve_liquidity_supply,
            lending_market,
            lending_market_authority,
            authority,
            clock,
            destination_liquidity,
        ],
        signers_seeds,
    )
}

pub fn init_obligation<'a>(
    program_id: &Pubkey,
    obligation_account: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    obligation_owner: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    rent: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = solend_program::instruction::init_obligation(
        *program_id,
        *obligation_account.key,
        *lending_market.key,
        *obligation_owner.key,
    );

    invoke_signed(
        &instruction,
        &[
            obligation_account,
            lending_market,
            obligation_owner,
            clock,
            rent,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_obligation_collateral<'a>(
    program_id: &Pubkey,
    source_collateral: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    deposit_reserve: AccountInfo<'a>,
    obligation: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    obligation_owner: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    collateral_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = solend_program::instruction::deposit_obligation_collateral(
        *program_id,
        collateral_amount,
        *source_collateral.key,
        *destination_collateral.key,
        *deposit_reserve.key,
        *obligation.key,
        *lending_market.key,
        *obligation_owner.key,
        *user_transfer_authority.key,
    );

    invoke_signed(
        &instruction,
        &[
            source_collateral,
            destination_collateral,
            deposit_reserve,
            obligation,
            lending_market,
            obligation_owner,
            lending_market_authority,
            clock,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_obligation_collateral<'a>(
    program_id: &Pubkey,
    source_collateral: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    withdraw_reserve: AccountInfo<'a>,
    obligation: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    obligation_owner: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    collateral_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction = solend_program::instruction::withdraw_obligation_collateral(
        *program_id,
        collateral_amount,
        *source_collateral.key,
        *destination_collateral.key,
        *withdraw_reserve.key,
        *obligation.key,
        *lending_market.key,
        *obligation_owner.key,
    );

    invoke_signed(
        &instruction,
        &[
            source_collateral,
            destination_collateral,
            withdraw_reserve,
            obligation,
            lending_market,
            lending_market_authority,
            obligation_owner,
            clock,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_reserve_liquidity_and_obligation_collateral<'a>(
    program_id: &Pubkey,
    source_liquidity: AccountInfo<'a>,
    user_collateral: AccountInfo<'a>,
    deposit_reserve: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    reserve_collateral_mint: AccountInfo<'a>,
    obligation: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    obligation_owner: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    reserve_liquidity_pyth_oracle: AccountInfo<'a>,
    reserve_liquidity_switchboard_oracle: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    collateral_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction =
        solend_program::instruction::deposit_reserve_liquidity_and_obligation_collateral(
            *program_id,
            collateral_amount,
            *source_liquidity.key,
            *user_collateral.key,
            *deposit_reserve.key,
            *reserve_liquidity_supply.key,
            *reserve_collateral_mint.key,
            *lending_market.key,
            *destination_collateral.key,
            *obligation.key,
            *obligation_owner.key,
            *reserve_liquidity_pyth_oracle.key,
            *reserve_liquidity_switchboard_oracle.key,
            *user_transfer_authority.key,
        );

    invoke_signed(
        &instruction,
        &[
            source_liquidity,
            user_collateral,
            deposit_reserve,
            reserve_liquidity_supply,
            reserve_collateral_mint,
            lending_market,
            lending_market_authority,
            destination_collateral,
            obligation,
            obligation_owner,
            reserve_liquidity_pyth_oracle,
            reserve_liquidity_switchboard_oracle,
            user_transfer_authority,
            clock,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_obligation_collateral_and_redeem_reserve_collateral<'a>(
    program_id: &Pubkey,
    source_collateral: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    withdraw_reserve: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    reserve_collateral_mint: AccountInfo<'a>,
    obligation: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    destination_liquidity: AccountInfo<'a>,
    obligation_owner: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    collateral_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let instruction =
        solend_program::instruction::withdraw_obligation_collateral_and_redeem_reserve_collateral(
            *program_id,
            collateral_amount,
            *source_collateral.key,
            *destination_collateral.key,
            *withdraw_reserve.key,
            *obligation.key,
            *lending_market.key,
            *destination_liquidity.key,
            *reserve_collateral_mint.key,
            *reserve_liquidity_supply.key,
            *obligation_owner.key,
            *user_transfer_authority.key,
        );

    invoke_signed(
        &instruction,
        &[
            source_collateral,
            destination_collateral,
            withdraw_reserve,
            obligation,
            lending_market,
            lending_market_authority,
            destination_liquidity,
            reserve_collateral_mint,
            reserve_liquidity_supply,
            obligation_owner,
            user_transfer_authority,
            clock,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn refresh_obligation<'a>(
    program_id: &Pubkey,
    obligation: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    clock: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let instruction = solend_program::instruction::refresh_obligation(
        *program_id,
        *obligation.key,
        vec![*reserve.key],
    );

    invoke(&instruction, &[obligation, reserve, clock])
}

pub fn claim_rewards<'a>(
    program_id: &Pubkey,
    distributor: AccountInfo<'a>,
    claim_status: AccountInfo<'a>,
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    claimant: AccountInfo<'a>,
    payer: AccountInfo<'a>,
    claim_data: ClaimData,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    struct Claim {
        instruction: [u8; 8],
        bump: u8,
        index: u64,
        amount: u64,
        proof: [u8; 32],
    }

    let instruction = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*distributor.key, false),
            AccountMeta::new(*claim_status.key, false),
            AccountMeta::new(*from.key, false),
            AccountMeta::new(*to.key, false),
            AccountMeta::new_readonly(*claimant.key, true),
            AccountMeta::new_readonly(*payer.key, true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: Claim {
            instruction: CLAIM_INSTRUCTION,
            bump: claim_data.bump,
            index: claim_data.index,
            amount: claim_data.amount,
            proof: claim_data.proof,
        }
        .try_to_vec()?,
    };

    let accounts = [distributor, claim_status, from, to, claimant, payer];

    invoke_signed(&instruction, &accounts, signers_seeds)
}
