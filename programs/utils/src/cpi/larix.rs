use larix_lending::instruction::LendingInstruction;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::{
    account_info::AccountInfo,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn refresh_reserves<'a>(
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

pub fn refresh_mining<'a>(
    program_id: &Pubkey,
    mining_info: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*mining_info.key, false),
            AccountMeta::new_readonly(*reserve.key, false),
        ],
        data: LendingInstruction::RefreshMining.pack(),
    };

    invoke(&ix, &[mining_info, reserve])
}

pub fn refresh_obligation<'a>(
    program_id: &Pubkey,
    obligation_info: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let ix = larix_lending::instruction::refresh_obligation(
        *program_id,
        *obligation_info.key,
        vec![*reserve.key],
    );

    invoke(&ix, &[obligation_info, reserve])
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

pub fn deposit_mining<'a>(
    program_id: &Pubkey,
    destination_collateral: AccountInfo<'a>,
    // Contains is reserve account ...bonus.unCollSupply
    reserve_bonus: AccountInfo<'a>,
    mining_info: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    mining_owner: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    // Use u64::MAX for depositing 100% of available amount
    amount: u64,
) -> Result<(), ProgramError> {
    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*destination_collateral.key, false),
            AccountMeta::new(*reserve_bonus.key, false),
            AccountMeta::new(*mining_info.key, false),
            AccountMeta::new_readonly(*reserve.key, false),
            AccountMeta::new_readonly(*lending_market.key, false),
            AccountMeta::new_readonly(*mining_owner.key, false),
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: LendingInstruction::DepositMining { amount }.pack(),
    };

    invoke(
        &ix,
        &[
            destination_collateral,
            reserve_bonus,
            mining_info,
            reserve,
            lending_market,
            mining_owner,
            authority,
        ],
    )
}

pub fn claim_mine<'a>(
    program_id: &Pubkey,
    destination_collateral: AccountInfo<'a>,
    mining_info: AccountInfo<'a>,
    obligation_info: Option<AccountInfo<'a>>,
    reserve: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    // claim times of user expected got: 100 equals 100%
    claim_times: u16,
    // the ratio of claim user's all mine token 10000 equals 100%
    claim_ratio: u16,
) -> Result<(), ProgramError> {
    const OBLIGATION_ACCOUNT_INDEX: usize = 5;

    let mut accounts_meta = vec![
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(*lending_market.key, false),
        AccountMeta::new_readonly(*lending_market_authority.key, false),
        AccountMeta::new_readonly(*authority.key, true),
        AccountMeta::new(*mining_info.key, false),
        AccountMeta::new(*destination_collateral.key, false),
        AccountMeta::new_readonly(*reserve.key, false),
    ];

    let mut accounts = vec![
        lending_market,
        lending_market_authority,
        authority,
        mining_info,
        destination_collateral,
        reserve,
    ];

    if let Some(obligation_info) = obligation_info {
        accounts_meta.insert(
            OBLIGATION_ACCOUNT_INDEX,
            AccountMeta::new_readonly(*obligation_info.key, false),
        );
        accounts.insert(OBLIGATION_ACCOUNT_INDEX, obligation_info);
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: accounts_meta,
        data: LendingInstruction::ClaimMine {
            claim_times,
            claim_ratio,
        }
        .pack(),
    };

    invoke(&ix, &accounts)
}
