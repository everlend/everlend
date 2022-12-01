//! Utils

use crate::money_market::{CollateralPool, CollateralStorage, Francium, MoneyMarket, Tulip};
use crate::money_market::{Frakt, Jet, Larix, PortFinance, SPLLending, Solend};
use crate::{
    state::{InternalMining, MiningType},
    TransitPDA,
};
use everlend_collateral_pool::find_pool_withdraw_authority_program_address;
use everlend_income_pools::utils::IncomePoolAccounts;
use everlend_registry::state::DistributionPubkeys;
use everlend_utils::{
    abs_diff, assert_account_key, cpi, find_program_address, integrations, AccountLoader,
    EverlendError, PDA,
};
use num_traits::Zero;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, instruction::AccountMeta, msg,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::Account;
use std::{cmp::Ordering, iter::Enumerate, slice::Iter};

const RESERVE_THRESHOLD: u64 = 20;

/// Deposit
#[allow(clippy::too_many_arguments)]
pub fn deposit<'a, 'b>(
    collateral_transit: &'a AccountInfo<'b>,
    collateral_mint: &'a AccountInfo<'b>,
    liquidity_transit: &'a AccountInfo<'b>,
    authority: &'a AccountInfo<'b>,
    clock: &'a AccountInfo<'b>,
    money_market: &Box<dyn MoneyMarket<'b> + 'a>,
    is_mining: bool,
    collateral_storage: Option<Box<dyn CollateralStorage<'b> + 'a>>,
    liquidity_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<u64, ProgramError> {
    if liquidity_amount.is_zero() {
        return Ok(0);
    }

    let collateral_amount = if is_mining {
        msg!("Deposit to Money market and deposit Mining");
        money_market.money_market_deposit_and_deposit_mining(
            collateral_mint.clone(),
            liquidity_transit.clone(),
            collateral_transit.clone(),
            authority.clone(),
            clock.clone(),
            liquidity_amount,
            signers_seeds,
        )?
    } else {
        msg!("Deposit to Money market");
        let collateral_amount = money_market.money_market_deposit(
            collateral_mint.clone(),
            liquidity_transit.clone(),
            collateral_transit.clone(),
            authority.clone(),
            clock.clone(),
            liquidity_amount,
            signers_seeds,
        )?;

        if collateral_amount == 0 {
            if money_market.is_collateral_return() {
                return Err(EverlendError::CollateralLeak.into());
            }

            // For money markets that do not return collateral tokens,
            // the collateral amount should be returned as a liquidity amount
            return Ok(liquidity_amount);
        }

        msg!("Deposit into collateral pool");
        if collateral_storage.is_none() {
            return Err(ProgramError::InvalidArgument);
        }

        collateral_storage.unwrap().deposit_collateral_tokens(
            collateral_transit.clone(),
            authority.clone(),
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )?;

        collateral_amount
    };

    Ok(collateral_amount)
}

/// Withdraw
#[allow(clippy::too_many_arguments)]
pub fn withdraw<'a, 'b>(
    income_pool_accounts: IncomePoolAccounts<'a, 'b>,
    collateral_transit: &'a AccountInfo<'b>,
    collateral_mint: &'a AccountInfo<'b>,
    liquidity_transit: &'a AccountInfo<'b>,
    liquidity_reserve_transit: &'a AccountInfo<'b>,
    authority: &'a AccountInfo<'b>,
    clock: &'a AccountInfo<'b>,
    money_market: &Box<dyn MoneyMarket<'b> + 'a>,
    is_mining: bool,
    collateral_storage: &Option<Box<dyn CollateralStorage<'b> + 'a>>,
    collateral_amount: u64,
    expected_liquidity_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let liquidity_transit_supply = Account::unpack(&liquidity_transit.data.borrow())?.amount;

    if is_mining {
        msg!("Withdraw from Mining and Redeem from Money market");
        money_market.money_market_redeem_and_withdraw_mining(
            collateral_mint.clone(),
            collateral_transit.clone(),
            liquidity_transit.clone(),
            authority.clone(),
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )?;
    } else {
        if money_market.is_collateral_return() {
            msg!("Withdraw from collateral pool");

            if collateral_storage.is_none() {
                return Err(ProgramError::InvalidArgument);
            }

            collateral_storage
                .as_ref()
                .unwrap()
                .withdraw_collateral_tokens(
                    collateral_transit.clone(),
                    authority.clone(),
                    clock.clone(),
                    collateral_amount,
                    signers_seeds,
                )?;
        }

        msg!("Redeem from Money market");
        money_market.money_market_redeem(
            collateral_mint.clone(),
            collateral_transit.clone(),
            liquidity_transit.clone(),
            authority.clone(),
            clock.clone(),
            collateral_amount,
            signers_seeds,
        )?;
    };

    let received_amount = Account::unpack(&liquidity_transit.data.borrow())?
        .amount
        .checked_sub(liquidity_transit_supply)
        .ok_or(EverlendError::MathOverflow)?;
    msg!("received_amount: {}", received_amount);
    msg!("expected_liquidity_amount: {}", expected_liquidity_amount);

    // Received liquidity amount may be less
    // https://blog.neodyme.io/posts/lending_disclosure
    let diff = abs_diff(received_amount, expected_liquidity_amount)?;

    // Deposit to income pool if income amount > 0
    match received_amount.cmp(&expected_liquidity_amount) {
        Ordering::Greater => {
            msg!("income_amount: {}", diff);
            everlend_income_pools::cpi::deposit(
                income_pool_accounts,
                liquidity_transit.clone(),
                authority.clone(),
                diff,
                signers_seeds,
            )?;
        }
        Ordering::Less => {
            msg!("income_amount: -{}", diff);
            if diff.gt(&RESERVE_THRESHOLD) {
                // throw error,  this amount is too big, probably something is wrong
                return Err(EverlendError::ReserveThreshold.into());
            }

            cpi::spl_token::transfer(
                liquidity_reserve_transit.clone(),
                liquidity_transit.clone(),
                authority.clone(),
                diff,
                signers_seeds,
            )?;
        }
        Ordering::Equal => {}
    }

    Ok(())
}

/// Money market
pub fn money_market<'a, 'b>(
    market: integrations::MoneyMarket,
    program_id: &Pubkey,
    money_market_program: &AccountInfo<'b>,
    money_market_account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    internal_mining: &AccountInfo<'b>,
    collateral_token_mint: &Pubkey,
    depositor_authority: &Pubkey,
    depositor: &Pubkey,
    liquidity_mint: &'a AccountInfo<'b>,
) -> Result<(Box<dyn MoneyMarket<'b> + 'a>, bool), ProgramError> {
    let internal_mining_type = if internal_mining.owner == program_id {
        Some(InternalMining::unpack(&internal_mining.data.borrow())?.mining_type)
    } else {
        None
    };

    if market.eq(&Default::default()) {
        return Err(EverlendError::InvalidRebalancingMoneyMarket.into());
    }

    let is_mining =
        internal_mining_type.is_some() && internal_mining_type != Some(MiningType::None);

    // Only for tests
    if money_market_program.key.to_string() == integrations::SPL_TOKEN_LENDING_PROGRAM_ID {
        let spl = SPLLending::init(*money_market_program.key, money_market_account_info_iter)?;
        return Ok((Box::new(spl), is_mining));
    }

    match market {
        integrations::MoneyMarket::PortFinance {
            money_market_program_id,
        } => {
            assert_account_key(money_market_program, &money_market_program_id)?;

            let port = PortFinance::init(
                *money_market_program.key,
                money_market_account_info_iter,
                internal_mining_type,
                collateral_token_mint,
                depositor_authority,
            )?;
            Ok((Box::new(port), is_mining))
        }
        integrations::MoneyMarket::Larix {
            money_market_program_id,
        } => {
            assert_account_key(money_market_program, &money_market_program_id)?;

            let larix = Larix::init(
                *money_market_program.key,
                money_market_account_info_iter,
                internal_mining_type,
            )?;
            Ok((Box::new(larix), is_mining))
        }
        integrations::MoneyMarket::Solend {
            money_market_program_id,
            ..
        } => {
            assert_account_key(money_market_program, &money_market_program_id)?;

            let solend = Solend::init(*money_market_program.key, money_market_account_info_iter)?;
            Ok((Box::new(solend), is_mining))
        }
        integrations::MoneyMarket::Tulip {
            money_market_program_id,
        } => {
            assert_account_key(money_market_program, &money_market_program_id)?;

            let tulip = Tulip::init(
                money_market_program.key.clone(),
                money_market_account_info_iter,
            )?;
            Ok((Box::new(tulip), is_mining))
        }
        integrations::MoneyMarket::Francium {
            money_market_program_id,
        } => {
            assert_account_key(money_market_program, &money_market_program_id)?;

            let francium = Francium::init(
                program_id,
                money_market_program.key.clone(),
                money_market_account_info_iter,
                depositor,
                depositor_authority,
                internal_mining_type,
            )?;
            Ok((Box::new(francium), is_mining))
        }
        integrations::MoneyMarket::Jet {
            money_market_program_id,
        } => {
            assert_account_key(money_market_program, &money_market_program_id)?;

            let jet = Jet::init(
                money_market_program.key.clone(),
                money_market_account_info_iter,
            )?;
            return Ok((Box::new(jet), is_mining));
        }
        integrations::MoneyMarket::Frakt {
            money_market_program_id,
            liquidity_pool,
        } => {
            assert_account_key(money_market_program, &money_market_program_id)?;

            let frakt = Frakt::init(
                money_market_program.key.clone(),
                program_id.clone(),
                depositor_authority,
                liquidity_mint,
                money_market_account_info_iter,
                &liquidity_pool,
            )?;
            Ok((Box::new(frakt), is_mining))
        }
    }
}

/// Money market
pub fn collateral_storage<'a, 'b>(
    collateral_pool_markets: DistributionPubkeys,
    collateral_mint: &AccountInfo<'b>,
    depositor_authority: &AccountInfo<'b>,
    account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    if_withdraw_expected: bool,
    is_mining: bool,
) -> Result<Option<Box<dyn CollateralStorage<'b> + 'a>>, ProgramError> {
    if is_mining {
        return Ok(None);
    };

    let coll_pool = CollateralPool::init(
        collateral_pool_markets,
        collateral_mint,
        depositor_authority,
        account_info_iter,
        if_withdraw_expected,
    )?;

    Ok(Some(Box::new(coll_pool)))
}

/// Collateral pool deposit account
#[allow(clippy::too_many_arguments)]
pub fn collateral_pool_deposit_accounts(
    pool_market: &Pubkey,
    collateral_mint: &Pubkey,
    collateral_pool_token_account: &Pubkey,
) -> Vec<AccountMeta> {
    let (collateral_pool_market_authority, _) =
        find_program_address(&everlend_collateral_pool::id(), pool_market);
    let (collateral_pool, _) = everlend_collateral_pool::find_pool_program_address(
        &everlend_collateral_pool::id(),
        pool_market,
        collateral_mint,
    );

    vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(collateral_pool_market_authority, false),
        AccountMeta::new_readonly(collateral_pool, false),
        AccountMeta::new(*collateral_pool_token_account, false),
        AccountMeta::new_readonly(everlend_collateral_pool::id(), false),
    ]
}

/// Collateral pool deposit account
#[allow(clippy::too_many_arguments)]
pub fn collateral_pool_withdraw_accounts(
    pool_market: &Pubkey,
    collateral_mint: &Pubkey,
    collateral_pool_token_account: &Pubkey,
    depositor_program_id: &Pubkey,
    depositor: &Pubkey,
) -> Vec<AccountMeta> {
    let (collateral_pool_market_authority, _) =
        find_program_address(&everlend_collateral_pool::id(), pool_market);
    let (collateral_pool, _) = everlend_collateral_pool::find_pool_program_address(
        &everlend_collateral_pool::id(),
        pool_market,
        collateral_mint,
    );

    let (depositor_authority, _) = find_program_address(depositor_program_id, depositor);

    let (collateral_pool_withdraw_authority, _) = find_pool_withdraw_authority_program_address(
        &everlend_collateral_pool::id(),
        &collateral_pool,
        &depositor_authority,
    );

    vec![
        AccountMeta::new_readonly(*pool_market, false),
        AccountMeta::new_readonly(collateral_pool_market_authority, false),
        AccountMeta::new_readonly(collateral_pool, false),
        AccountMeta::new(*collateral_pool_token_account, false),
        AccountMeta::new_readonly(everlend_collateral_pool::id(), false),
        AccountMeta::new_readonly(collateral_pool_withdraw_authority, false),
    ]
}

/// ELD Fill reward accounts for token container
#[derive(Clone)]
pub struct FillRewardAccounts<'a, 'b> {
    /// Rewards mint tokne
    pub reward_mint_info: &'a AccountInfo<'b>,
    /// Reward transit account
    pub reward_transit_info: &'a AccountInfo<'b>,
    /// Reward vault
    pub vault_info: &'a AccountInfo<'b>,
    /// Reward fee account
    pub fee_account_info: &'a AccountInfo<'b>,
}

impl<'a, 'b> FillRewardAccounts<'a, 'b> {
    ///
    // Check rewards destination account only if needed
    pub fn check_transit_reward_destination(
        &self,
        program_id: &Pubkey,
        depositor_id: &Pubkey,
    ) -> Result<(), ProgramError> {
        let (reward_token_account, _) = TransitPDA {
            seed: "lm_reward",
            depositor: depositor_id.clone(),
            mint: *self.reward_mint_info.key,
        }
        .find_address(program_id);
        assert_account_key(self.reward_transit_info, &reward_token_account)?;

        Ok(())
    }
}

/// Collateral pool deposit account
#[allow(clippy::too_many_arguments)]
pub fn parse_fill_reward_accounts<'a, 'b>(
    reward_pool_id: &Pubkey,
    eld_reward_program_id: &Pubkey,
    account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
) -> Result<FillRewardAccounts<'a, 'b>, ProgramError> {
    let reward_mint_info = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
    let reward_transit_info = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

    let vault_info = AccountLoader::next_unchecked(account_info_iter)?;
    let fee_account_info = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

    let (vault, _) = Pubkey::find_program_address(
        &[
            b"vault".as_ref(),
            &reward_pool_id.to_bytes(),
            &reward_mint_info.key.to_bytes(),
        ],
        eld_reward_program_id,
    );
    assert_account_key(vault_info, &vault)?;

    Ok(FillRewardAccounts {
        reward_mint_info,
        reward_transit_info,
        vault_info,
        fee_account_info,
    })
}

/// Calculates available liquidity and amount to distribute
pub fn calculate_amount_to_distribute(
    total_distributed_liquidity: u64,
    liquidity_transit: u64,
    general_pool_amount: u64,
    withdrawal_requests: u64,
) -> Result<(u64, u64), ProgramError> {
    let available_liquidity = total_distributed_liquidity
        .checked_add(liquidity_transit)
        .ok_or(EverlendError::MathOverflow)?;

    // Calculate liquidity to distribute
    let amount_to_distribute = general_pool_amount
        .checked_add(available_liquidity)
        .ok_or(EverlendError::MathOverflow)?
        .checked_sub(withdrawal_requests)
        .ok_or(EverlendError::MathOverflow)?;

    Ok((available_liquidity, amount_to_distribute))
}
