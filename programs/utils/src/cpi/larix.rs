use larix_lending::instruction::LendingInstruction;
use larix_lending::math::{Decimal, TryAdd, TryDiv, TrySub};
use solana_program::program_pack::Pack;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
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
            AccountMeta::new(*mining_info.key, false),
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
            AccountMeta::new(*reserve_bonus.key, false),
            AccountMeta::new(*source_collateral.key, false),
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
            reserve_bonus,
            source_collateral,
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
    mining: AccountInfo<'a>,
    mine_supply: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let accounts_meta = vec![
        AccountMeta::new(*mining.key, false),
        AccountMeta::new(*mine_supply.key, false),
        AccountMeta::new(*destination.key, false),
        AccountMeta::new_readonly(*authority.key, true),
        AccountMeta::new_readonly(*lending_market.key, false),
        AccountMeta::new_readonly(*lending_market_authority.key, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(*reserve.key, false),
    ];

    let accounts = vec![
        lending_market,
        lending_market_authority,
        authority,
        mining,
        mine_supply,
        destination,
        reserve,
    ];

    let ix = Instruction {
        program_id: *program_id,
        accounts: accounts_meta,
        data: LendingInstruction::ClaimMiningMine.pack(),
    };

    invoke_signed(&ix, &accounts, signers_seeds)
}

#[allow(clippy::too_many_arguments)]
pub fn refresh_mine<'a>(
    program_id: &Pubkey,
    mining: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
) -> ProgramResult {
    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*mining.key, false),
            AccountMeta::new_readonly(*reserve.key, false),
        ],
        data: LendingInstruction::RefreshMining.pack(),
    };

    invoke(&ix, &[mining, reserve])
}

pub fn get_real_liquidity_amount(
    reserve: AccountInfo,
    collateral_amount: u64,
) -> Result<u64, ProgramError> {
    let reserve = larix_lending::state::reserve::Reserve::unpack(&reserve.data.borrow())?;

    let total_asset = Decimal::from(reserve.liquidity.available_amount)
        .try_add(reserve.liquidity.borrowed_amount_wads)?
        .try_sub(reserve.liquidity.owner_unclaimed)?;
    let rate = Decimal::from(reserve.collateral.mint_total_supply).try_div(total_asset)?;

    Decimal::from(collateral_amount)
        .try_div(rate)?
        .try_floor_u64()
}
