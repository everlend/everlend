//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use everlend_general_pool::find_withdrawal_requests_program_address;
use everlend_liquidity_oracle::{find_token_oracle_program_address, state::DistributionArray};
use everlend_utils::cpi::{quarry,francium};
use everlend_utils::find_program_address;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};
use spl_associated_token_account::get_associated_token_address;

use crate::{find_rebalancing_program_address, find_transit_program_address, state::MiningType};

/// Instructions supported by the program
#[derive(Debug, BorshDeserialize, BorshSerialize, PartialEq)]
pub enum DepositorInstruction {
    /// Initializes a new depositor
    ///
    /// Accounts:
    /// [W] Depositor account - uninitialized
    /// [R] Registry
    /// [R] Rent sysvar
    Init {
        /// Rebalance executor account
        rebalance_executor: Pubkey,
    },

    /// Create transit token account for liquidity
    ///
    /// Accounts:
    /// [R] Depositor account
    /// [R] Depositor authority
    /// [W] Transit account
    /// [R] Token mint
    /// [WS] From account
    /// [R] Rent sysvar
    /// [R] System program
    /// [R] Token program id
    CreateTransit {
        /// Seed
        seed: String,
    },

    /// Computing rebalancing steps and updating the liquidity on the transit account
    ///
    /// Accounts:
    /// [R] Registry config
    /// [R] Depositor
    /// [R] Depositor authority
    /// [W] Rebalancing account
    /// [R] Token mint
    /// [R] General pool market
    /// [R] General pool market authority
    /// [W] General pool
    /// [W] General pool token account
    /// [W] General pool borrow authority
    /// [R] Withdrawals requests account
    /// [W] Liquidity transit account
    /// [R] Liquidity oracle
    /// [R] Token distribution
    /// [WS] Rebalance executor account
    /// [R] Rent sysvar
    /// [R] Clock sysvar
    /// [R] System program
    /// [R] Token program id
    /// [R] Everlend general pool program id
    StartRebalancing {
        /// Refresh income
        refresh_income: bool,
    },

    /// Deposit funds from liquidity transit account to money market.
    /// Collect collateral token to MM pool.
    ///
    /// Accounts:
    /// [R] Registry config
    /// [R] Depositor
    /// [R] Depositor authority
    /// [W] Rebalancing account
    /// [W] Liquidity transit account
    /// [R] Liquidity mint
    /// [W] Collateral transit account
    /// [W] Collateral mint
    /// [S] Rebalance executor account
    /// [R] Clock sysvar
    /// [R] Token program id
    /// [R] Money market program id
    /// [R] Internal mining account
    /// [] Money market deposit accounts
    /// [] Collateral storage accounts or money market mining accounts
    Deposit,

    /// Withdraw funds from MM pool to money market.
    /// Collect liquidity token to liquidity transit account.
    ///
    /// Accounts:
    /// [R] Registry config
    /// [R] Depositor
    /// [R] Depositor authority
    /// [W] Rebalancing account
    /// [R] Income pool market
    /// [R] Income pool
    /// [W] Income pool token account (for liquidity mint)
    /// [W] Collateral transit account
    /// [W] Collateral mint
    /// [W] Liquidity transit account
    /// [W] Liquidity reserve transit account
    /// [R] Liquidity mint
    /// [S] Rebalance executor account
    /// [R] Clock sysvar
    /// [R] Token program id
    /// [R] Money market program id
    /// [R] Internal mining account
    /// [] Money market deposit accounts
    /// [] Collateral storage accounts or money market mining accounts
    Withdraw,

    /// Initialize account for mining LM rewards
    ///
    /// Accounts:
    /// [W] Internal mining account
    /// [R] Liquidity mint
    /// [R] Collateral mint (collateral of liquidity asset)
    /// [R] Depositor
    /// [R] Depositor authority
    /// [R] Registry
    /// [WS] Manager
    /// [R] Rent sysvar
    /// [R] System program
    /// For larix mining:
    /// [R] Mining program ID
    /// [W] Mining account
    /// [R] Lending market
    /// [R] Optional: Additional reward token account
    /// For PortFinance mining:
    /// [R] Staking program id
    /// [R] Staking pool
    /// [W] Staking account
    /// [R] Money market program ID
    /// [W] Obligation account
    /// [R] Lending market
    /// [R] Clock sysvar
    /// [R] Token program ID
    /// For PortFinanceQuarry:
    /// [R] Staking program id
    /// [R] Rewarder
    /// [W] Quarry
    /// [W] Miner account
    /// [R] Miner vault
    /// [R] Token program ID
    InitMiningAccount {
        /// Type of mining
        mining_type: MiningType,
    },

    /// Claim mining reward
    ///
    /// Accounts:
    /// [R] Depositor
    /// [R] Depositor authority
    /// [S] Executor
    /// [R] Liquidity mint
    /// [R] Collateral mint
    /// [R] Internal mining account
    ///
    /// [R] Token program id
    /// [R] Staking program id
    /// [R] ELD reward program id
    /// [W] Reward pool
    /// Reward fill accounts
    /// [R] Reward mint
    /// [W] Reward transit account
    /// [W] Vault
    /// [W] Vault fee account
    /// If mining has subreward add `Reward fill accounts` for subreward token
    /// For larix mining:
    /// [W] Mining account
    /// [W] Mine supply
    /// [W] Destination collateral
    /// [R] Lending market
    /// [R] Lending market authority
    /// [R] Reserve
    /// [R] Reserve liquidity oracle
    /// For PortFinance mining:
    /// [R] Stake account owner
    /// [W] Stake account
    /// [W] Staking pool
    /// [W] Reward token pool
    /// [W] Reward destination
    /// [R] Sub reward token pool
    /// [R] Sub reward destination
    /// For Quarry mining:
    /// [W] Mint wrapper
    /// [R] Mint wrapper program
    /// [W] Minter
    /// [W] Rewards token mint
    /// [W] Rewards token account
    /// [W] Claim fee token account
    /// [W] Miner
    /// [W] Quarry
    /// [R] Rewarder
    ClaimMiningReward {
        ///
        with_subrewards: bool,
    },

    /// Migrate Depositor
    ///
    /// Accounts:
    /// [W] Depositor
    /// [S] Manager
    MigrateDepositor,

    /// Set current rebalancing
    ///
    /// Accounts:
    /// [R] Registry
    /// [R] Depositor
    /// [W] Rebalancing account
    /// [R] Token mint
    /// [S] Manager
    SetRebalancing {
        /// Manual setup of amount to distribute
        amount_to_distribute: u64,
        /// Manual setup of prev distributed liquidity
        distributed_liquidity: DistributionArray,
        /// Manual setup of prev distribution array
        distribution_array: DistributionArray,
    },

    /// Refresh incomes for MM
    /// Withdraw funds from MM pool and deposit back to charge rewards.
    ///
    /// Accounts:
    /// [R] Registry config
    /// [R] Depositor
    /// [R] Depositor authority
    /// [W] Rebalancing account
    /// [R] Income pool market
    /// [R] Income pool
    /// [W] Income pool token account (for liquidity mint)
    /// [W] Collateral transit account
    /// [W] Collateral mint
    /// [W] Liquidity transit account
    /// [W] Liquidity reserve transit account
    /// [R] Liquidity mint
    /// [S] Rebalance executor account
    /// [R] Clock sysvar
    /// [R] Token program id
    /// [R] Money market program id
    /// [R] Internal mining account
    /// [] Money market deposit accounts
    /// [] Collateral storage accounts or money market mining accounts
    RefreshMMIncomes,
}

/// Creates 'Init' instruction.
#[allow(clippy::too_many_arguments)]
pub fn init(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    rebalance_executor: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*depositor, false),
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::Init {
            rebalance_executor: *rebalance_executor,
        },
        accounts,
    )
}

/// Creates 'CreateTransit' instruction.
#[allow(clippy::too_many_arguments)]
pub fn create_transit(
    program_id: &Pubkey,
    depositor: &Pubkey,
    mint: &Pubkey,
    signer: &Pubkey,
    seed: Option<String>,
) -> Instruction {
    let seed = seed.unwrap_or_default();
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (transit, _) = find_transit_program_address(program_id, depositor, mint, &seed);

    let accounts = vec![
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(transit, false),
        AccountMeta::new_readonly(*mint, false),
        // Any account
        AccountMeta::new(*signer, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::CreateTransit { seed },
        accounts,
    )
}

/// Creates 'StartRebalancing' instruction.
#[allow(clippy::too_many_arguments)]
pub fn start_rebalancing(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    mint: &Pubkey,
    general_pool_market: &Pubkey,
    general_pool_token_account: &Pubkey,
    liquidity_oracle: &Pubkey,
    rebalance_executor: &Pubkey,
    refresh_income: bool,
) -> Instruction {
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, mint);
    let (token_oracle, _) =
        find_token_oracle_program_address(&everlend_liquidity_oracle::id(), liquidity_oracle, mint);
    // General pool
    let (general_pool_market_authority, _) =
        find_program_address(&everlend_general_pool::id(), general_pool_market);
    let (general_pool, _) = everlend_general_pool::find_pool_program_address(
        &everlend_general_pool::id(),
        general_pool_market,
        mint,
    );
    let (general_pool_borrow_authority, _) =
        everlend_general_pool::find_pool_borrow_authority_program_address(
            &everlend_general_pool::id(),
            &general_pool,
            &depositor_authority,
        );
    let (withdrawal_requests, _) = find_withdrawal_requests_program_address(
        &everlend_general_pool::id(),
        general_pool_market,
        mint,
    );

    let (liquidity_transit, _) = find_transit_program_address(program_id, depositor, mint, "");

    let accounts = vec![
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(rebalancing, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*general_pool_market, false),
        AccountMeta::new_readonly(general_pool_market_authority, false),
        AccountMeta::new(general_pool, false),
        AccountMeta::new(*general_pool_token_account, false),
        AccountMeta::new(general_pool_borrow_authority, false),
        AccountMeta::new_readonly(withdrawal_requests, false),
        AccountMeta::new(liquidity_transit, false),
        AccountMeta::new_readonly(*liquidity_oracle, false),
        AccountMeta::new_readonly(token_oracle, false),
        AccountMeta::new(*rebalance_executor, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(everlend_general_pool::id(), false),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::StartRebalancing { refresh_income },
        accounts,
    )
}

/// Creates 'ResetRebalancing' instruction.
#[allow(clippy::too_many_arguments)]
pub fn reset_rebalancing(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    liquidity_mint: &Pubkey,
    manager: &Pubkey,
    amount_to_distribute: u64,
    distributed_liquidity: DistributionArray,
    distribution_array: DistributionArray,
) -> Instruction {
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, liquidity_mint);

    let accounts = vec![
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new(rebalancing, false),
        AccountMeta::new_readonly(*liquidity_mint, false),
        AccountMeta::new_readonly(*manager, true),
    ];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::SetRebalancing {
            amount_to_distribute,
            distributed_liquidity,
            distribution_array,
        },
        accounts,
    )
}

/// Creates 'Deposit' instruction.
#[allow(clippy::too_many_arguments)]
pub fn deposit(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    liquidity_mint: &Pubkey,
    collateral_mint: &Pubkey,
    rebalance_executor: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
    collateral_storage_accounts: Vec<AccountMeta>,
) -> Instruction {
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, liquidity_mint);

    let (liquidity_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint, "");
    let (collateral_transit, _) =
        find_transit_program_address(program_id, depositor, collateral_mint, "");

    let (internal_mining, _) = crate::find_internal_mining_program_address(
        program_id,
        liquidity_mint,
        collateral_mint,
        depositor,
    );

    let mut accounts = vec![
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(rebalancing, false),
        // Common
        AccountMeta::new(liquidity_transit, false),
        AccountMeta::new_readonly(*liquidity_mint, false),
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*collateral_mint, false),
        AccountMeta::new_readonly(*rebalance_executor, true),
        // Programs
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        // Money market
        AccountMeta::new_readonly(*money_market_program_id, false),
        AccountMeta::new_readonly(internal_mining, false),
    ];

    accounts.extend(money_market_accounts);
    accounts.extend(collateral_storage_accounts);

    Instruction::new_with_borsh(*program_id, &DepositorInstruction::Deposit, accounts)
}

/// Creates 'Withdraw' instruction.
#[allow(clippy::too_many_arguments)]
pub fn withdraw(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    income_pool_market: &Pubkey,
    income_pool_token_account: &Pubkey,
    collateral_mint: &Pubkey,
    liquidity_mint: &Pubkey,
    rebalance_executor: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
    collateral_storage_accounts: Vec<AccountMeta>,
) -> Instruction {
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, liquidity_mint);

    // Income pool
    let (income_pool, _) = everlend_income_pools::find_pool_program_address(
        &everlend_income_pools::id(),
        income_pool_market,
        liquidity_mint,
    );

    let (collateral_transit, _) =
        find_transit_program_address(program_id, depositor, collateral_mint, "");
    let (liquidity_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint, "");

    let (liquidity_reserve_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint, "reserve");

    let (internal_mining, _internal_mining_bump_seed) = crate::find_internal_mining_program_address(
        program_id,
        liquidity_mint,
        collateral_mint,
        depositor,
    );

    let mut accounts = vec![
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(rebalancing, false),
        // Income pool
        AccountMeta::new_readonly(*income_pool_market, false),
        AccountMeta::new_readonly(income_pool, false),
        AccountMeta::new(*income_pool_token_account, false),
        // Common
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*collateral_mint, false),
        AccountMeta::new(liquidity_transit, false),
        AccountMeta::new(liquidity_reserve_transit, false),
        AccountMeta::new_readonly(*liquidity_mint, false),
        AccountMeta::new_readonly(*rebalance_executor, true),
        // Programs
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(everlend_income_pools::id(), false),
        // Money market
        AccountMeta::new_readonly(*money_market_program_id, false),
        AccountMeta::new_readonly(internal_mining, false),
    ];

    accounts.extend(money_market_accounts);
    accounts.extend(collateral_storage_accounts);

    Instruction::new_with_borsh(*program_id, &DepositorInstruction::Withdraw, accounts)
}

/// Creates 'RefreshMMIncomes' instruction.
#[allow(clippy::too_many_arguments)]
pub fn refresh_mm_incomes(
    program_id: &Pubkey,
    registry: &Pubkey,
    depositor: &Pubkey,
    income_pool_market: &Pubkey,
    income_pool_token_account: &Pubkey,
    collateral_mint: &Pubkey,
    liquidity_mint: &Pubkey,
    rebalance_executor: &Pubkey,
    money_market_program_id: &Pubkey,
    money_market_accounts: Vec<AccountMeta>,
    collateral_storage_accounts: Vec<AccountMeta>,
) -> Instruction {
    let (depositor_authority, _) = find_program_address(program_id, depositor);
    let (rebalancing, _) = find_rebalancing_program_address(program_id, depositor, liquidity_mint);

    // Income pool
    let (income_pool, _) = everlend_income_pools::find_pool_program_address(
        &everlend_income_pools::id(),
        income_pool_market,
        liquidity_mint,
    );

    let (collateral_transit, _) =
        find_transit_program_address(program_id, depositor, collateral_mint, "");
    let (liquidity_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint, "");

    let (liquidity_reserve_transit, _) =
        find_transit_program_address(program_id, depositor, liquidity_mint, "reserve");

    let (internal_mining, _internal_mining_bump_seed) = crate::find_internal_mining_program_address(
        program_id,
        liquidity_mint,
        collateral_mint,
        depositor,
    );

    let mut accounts = vec![
        AccountMeta::new_readonly(*registry, false),
        AccountMeta::new_readonly(*depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new(rebalancing, false),
        // Income pool
        AccountMeta::new_readonly(*income_pool_market, false),
        AccountMeta::new_readonly(income_pool, false),
        AccountMeta::new(*income_pool_token_account, false),
        // Common
        AccountMeta::new(collateral_transit, false),
        AccountMeta::new(*collateral_mint, false),
        AccountMeta::new(liquidity_transit, false),
        AccountMeta::new(liquidity_reserve_transit, false),
        AccountMeta::new_readonly(*liquidity_mint, false),
        AccountMeta::new_readonly(*rebalance_executor, true),
        // Programs
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(everlend_income_pools::id(), false),
        // Money market
        AccountMeta::new_readonly(*money_market_program_id, false),
        AccountMeta::new_readonly(internal_mining, false),
    ];

    accounts.extend(money_market_accounts);
    accounts.extend(collateral_storage_accounts);

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::RefreshMMIncomes,
        accounts,
    )
}

/// Creates 'MigrateDepositor' instruction.
#[allow(clippy::too_many_arguments)]
pub fn migrate_depositor(
    program_id: &Pubkey,
    _depositor: &Pubkey,
    _registry: &Pubkey,
    _manager: &Pubkey,
    _rebalancing: &Pubkey,
    _liquidity_mint: &Pubkey,
    _amount_to_distribute: u64,
) -> Instruction {
    let accounts = vec![];

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::MigrateDepositor,
        accounts,
    )
}

/// Argument to init_mining_accounts
pub struct InitMiningAccountsPubkeys {
    /// Liquidity mint
    pub liquidity_mint: Pubkey,
    /// Collateral mint
    pub collateral_mint: Pubkey,
    /// Money market program id
    pub money_market_program_id: Pubkey,
    /// Depositor
    pub depositor: Pubkey,
    /// Registry
    pub registry: Pubkey,
    /// Manager
    pub manager: Pubkey,
    /// Lending market
    pub lending_market: Option<Pubkey>,
}

/// Init mining account
pub fn init_mining_account(
    program_id: &Pubkey,
    pubkeys: &InitMiningAccountsPubkeys,
    mining_type: MiningType,
) -> Instruction {
    let (internal_mining, _) = crate::find_internal_mining_program_address(
        program_id,
        &pubkeys.liquidity_mint,
        &pubkeys.collateral_mint,
        &pubkeys.depositor,
    );

    let (depositor_authority, _) = find_program_address(program_id, &pubkeys.depositor);

    let mut accounts = vec![
        AccountMeta::new(internal_mining, false),
        AccountMeta::new_readonly(pubkeys.liquidity_mint, false),
        AccountMeta::new_readonly(pubkeys.collateral_mint, false),
        AccountMeta::new_readonly(pubkeys.depositor, false),
        AccountMeta::new_readonly(depositor_authority, false),
        AccountMeta::new_readonly(pubkeys.registry, false),
        AccountMeta::new(pubkeys.manager, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    match mining_type {
        MiningType::Larix {
            mining_account,
            additional_reward_token_account,
        } => {
            // Mining program equal to money_market_program_id
            accounts.push(AccountMeta::new_readonly(
                pubkeys.money_market_program_id,
                false,
            ));
            accounts.push(AccountMeta::new(mining_account, false));
            accounts.push(AccountMeta::new_readonly(
                pubkeys.lending_market.unwrap(),
                false,
            ));

            if let Some(additional_reward_token_account) = additional_reward_token_account {
                accounts.push(AccountMeta::new_readonly(
                    additional_reward_token_account,
                    false,
                ));
            }
        }
        MiningType::PortFinance {
            staking_program_id,
            staking_account,
            staking_pool,
            obligation,
        } => {
            accounts.push(AccountMeta::new_readonly(staking_program_id, false));
            accounts.push(AccountMeta::new_readonly(staking_pool, false));
            accounts.push(AccountMeta::new(staking_account, false));

            // Init obligation
            accounts.push(AccountMeta::new_readonly(
                pubkeys.money_market_program_id,
                false,
            ));
            accounts.push(AccountMeta::new(obligation, false));
            accounts.push(AccountMeta::new_readonly(
                pubkeys.lending_market.unwrap(),
                false,
            ));
            accounts.push(AccountMeta::new_readonly(sysvar::clock::id(), false));
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
        }
        MiningType::Quarry { rewarder } => {
            let (quarry, _) = quarry::find_quarry_program_address(
                &quarry::staking_program_id(),
                &rewarder,
                &pubkeys.collateral_mint,
            );
            let (miner_pubkey, _) = quarry::find_miner_program_address(
                &quarry::staking_program_id(),
                &quarry,
                &depositor_authority,
            );

            let miner_vault = get_associated_token_address(&miner_pubkey, &pubkeys.collateral_mint);

            accounts.push(AccountMeta::new_readonly(
                quarry::staking_program_id(),
                false,
            ));
            accounts.push(AccountMeta::new_readonly(rewarder, false));
            accounts.push(AccountMeta::new(quarry, false));
            accounts.push(AccountMeta::new(miner_pubkey, false));
            accounts.push(AccountMeta::new_readonly(miner_vault, false));

            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
        }
        MiningType::Francium {
            user_stake_token_account,
            farming_pool,
            user_reward_a,
            user_reward_b
        } => {
            let staking_program_id = francium::get_staking_program_id();

            let ( user_farming, _ ) = Pubkey::find_program_address(
                &[
                    depositor_authority.as_ref(),
                    farming_pool.as_ref(),
                    user_stake_token_account.as_ref()
                ],
                &staking_program_id,
            );

            accounts.push(AccountMeta::new_readonly(staking_program_id, false));
            accounts.push(AccountMeta::new(farming_pool, false));
            accounts.push(AccountMeta::new(user_farming, false));
            accounts.push(AccountMeta::new(user_reward_a, false));
            accounts.push(AccountMeta::new(user_reward_b, false));
            accounts.push(AccountMeta::new(user_stake_token_account, false));
        }
        MiningType::None => {}
    }

    Instruction::new_with_borsh(
        *program_id,
        &DepositorInstruction::InitMiningAccount { mining_type },
        accounts,
    )
}
