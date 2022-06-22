use anchor_lang::prelude::AccountMeta;
use larix_lending::instruction::LendingInstruction;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::Instruction,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn refresh_reserve<'a>(
    program_id: &Pubkey,
    reserve: AccountInfo<'a>,
    reserve_liquidity_oracle: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let ix = larix_lending::instruction::refresh_reserves(
        *program_id,
        vec![*reserve.key],
        vec![*reserve_liquidity_oracle.key],
    );

    invoke(&ix, &[reserve, reserve_liquidity_oracle])
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
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = larix_lending::instruction::deposit_reserve_liquidity(
        *program_id,
        amount,
        *source_liquidity.key,
        *destination_collateral.key,
        *reserve.key,
        *reserve_collateral_mint.key,
        *reserve_liquidity_supply.key,
        *lending_market.key,
        *lending_market_authority.key,
        *authority.key,
    );

    invoke_signed(
        &ix,
        &[
            source_liquidity,
            destination_collateral,
            reserve,
            reserve_collateral_mint,
            reserve_liquidity_supply,
            lending_market,
            lending_market_authority,
            authority,
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
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = larix_lending::instruction::redeem_reserve_collateral(
        *program_id,
        amount,
        *source_collateral.key,
        *destination_liquidity.key,
        *reserve.key,
        *reserve_collateral_mint.key,
        *reserve_liquidity_supply.key,
        *lending_market.key,
        *lending_market_authority.key,
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
            destination_liquidity,
        ],
        signers_seeds,
    )
}

pub fn init_mining<'a>(
    program_id: &Pubkey,
    // Random uninitialized lending program account for future Mining account
    mining_info: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*mining_info.key, false),
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new_readonly(*lending_market.key, false),
        ],
        data: LendingInstruction::InitMining.pack(),
    };

    invoke_signed(
        &ix,
        &[mining_info, authority, lending_market],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_mining<'a>(
    program_id: &Pubkey,
    source_collateral: AccountInfo<'a>,
    // Contains is reserve account ...bonus.unCollSupply
    reserve_bonus: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    mining_owner: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    // Use u64::MAX for depositing 100% of available amount
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*source_collateral.key, false),
            AccountMeta::new(*reserve_bonus.key, false),
            AccountMeta::new(*mining.key, false),
            AccountMeta::new_readonly(*reserve.key, false),
            AccountMeta::new_readonly(*lending_market.key, false),
            AccountMeta::new_readonly(*mining_owner.key, false),
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::DepositMining { amount }.pack(),
    };

    invoke_signed(
        &ix,
        &[
            source_collateral,
            reserve_bonus,
            mining,
            reserve,
            lending_market,
            mining_owner,
            authority,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw_mining<'a>(
    program_id: &Pubkey,
    source_collateral: AccountInfo<'a>,
    // Contains is reserve account ...bonus.unCollSupply
    reserve_bonus: AccountInfo<'a>,
    mining_info: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    mining_owner: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    // Use u64::MAX for depositing 100% of available amount
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*source_collateral.key, false),
            AccountMeta::new(*reserve_bonus.key, false),
            AccountMeta::new(*mining_info.key, false),
            AccountMeta::new_readonly(*reserve.key, false),
            AccountMeta::new_readonly(*lending_market.key, false),
            AccountMeta::new_readonly(*lending_market_authority.key, false),
            AccountMeta::new_readonly(*mining_owner.key, true),
            AccountMeta::new_readonly(*clock.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::WithdrawMining { amount }.pack(),
    };

    invoke_signed(
        &ix,
        &[
            source_collateral,
            reserve_bonus,
            mining_info,
            reserve,
            lending_market,
            lending_market_authority,
            mining_owner,
            clock,
        ],
        signers_seeds,
    )
}

pub struct ClaimMineAccounts<'a> {
    pub destination_collateral: AccountInfo<'a>,
    pub mining: AccountInfo<'a>,
    pub reserve: AccountInfo<'a>,
    pub lending_market: AccountInfo<'a>,
    pub lending_market_authority: AccountInfo<'a>,
    pub authority: AccountInfo<'a>,
}

#[allow(clippy::too_many_arguments)]
pub fn claim_mine<'a>(
    program_id: &Pubkey,
    destination: AccountInfo<'a>,
    mining: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let accounts_meta = vec![
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(*lending_market.key, false),
        AccountMeta::new_readonly(*lending_market_authority.key, false),
        AccountMeta::new_readonly(*authority.key, true),
        AccountMeta::new(*mining.key, false),
        // TODO ??? *Obligation account. After accounts pop if this account can not provided*
        AccountMeta::new(*destination.key, false),
        AccountMeta::new_readonly(*reserve.key, false),
    ];

    let accounts = vec![
        lending_market,
        lending_market_authority,
        authority,
        mining,
        destination,
        reserve,
    ];

    let ix = Instruction {
        program_id: *program_id,
        accounts: accounts_meta,
        data: LendingInstruction::ClaimMine {
            // claim times of user expected got: 100 equals 100%
            claim_times: 100,
            // the ratio of claim user's all mine token 10000 equals 100%
            claim_ratio: 10000,
        }
        .pack(),
    };

    invoke_signed(&ix, &accounts, signers_seeds)
}
