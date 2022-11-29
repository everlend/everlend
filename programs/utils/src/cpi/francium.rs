use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Slot;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar;
use spl_token_lending::math::{Decimal, TryAdd, TryDiv};
use std::str::FromStr;

pub const FRANCIUM_REWARD_SEED: &str = "francium_reward";

const DISABLED_DEPOSIT_VALUE: u8 = 0xFF;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct FarmingPool {
    pub version: u8,
    pub is_dual_rewards: bool,
    pub admin: Pubkey,
    pub pool_authority: Pubkey,
    pub token_program_id: Pubkey,

    // staked_token
    pub staked_token_mint: Pubkey,
    pub staked_token_account: Pubkey,

    // reward_token
    pub rewards_token_mint: Pubkey,
    pub rewards_token_account: Pubkey,

    // reward_token_b
    pub rewards_token_mint_b: Pubkey,
    pub rewards_token_account_b: Pubkey,

    // rewards config
    pub pool_stake_cap: u64,
    pub user_stake_cap: u64,
    // rewards a
    pub rewards_start_slot: Slot,
    pub rewards_end_slot: Slot,
    pub rewards_per_day: u64,

    // rewards b
    pub rewards_start_slot_b: Slot,
    pub rewards_end_slot_b: Slot,
    pub rewards_per_day_b: u64,

    pub total_staked_amount: u64,
    pub last_update_slot: Slot,

    pub accumulated_rewards_per_share: u128,
    pub accumulated_rewards_per_share_b: u128,
    pub padding: [u8; 128],
}

pub fn refresh_reserve(program_id: &Pubkey, reserve: AccountInfo) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct UpdateLendingPool {
        instruction: u8,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![AccountMeta::new(*reserve.key, false)],
        data: UpdateLendingPool { instruction: 17 }.try_to_vec()?,
    };

    invoke(&ix, &[reserve])
}

#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    program_id: &Pubkey,
    source_liquidity: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    reserve: AccountInfo<'a>,
    reserve_collateral_mint: AccountInfo<'a>,
    reserve_liquidity_supply: AccountInfo<'a>,
    lending_market: AccountInfo<'a>,
    lending_market_authority: AccountInfo<'a>,
    user_transfer_authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct DepositToLendingPool {
        instruction: u8,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*source_liquidity.key, false),
            AccountMeta::new(*destination_collateral.key, false),
            AccountMeta::new(*reserve.key, false),
            AccountMeta::new(*reserve_liquidity_supply.key, false),
            AccountMeta::new(*reserve_collateral_mint.key, false),
            AccountMeta::new_readonly(*lending_market.key, false),
            AccountMeta::new_readonly(*lending_market_authority.key, false),
            AccountMeta::new_readonly(*user_transfer_authority.key, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: DepositToLendingPool {
            instruction: 4,
            amount,
        }
        .try_to_vec()?,
    };

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
            user_transfer_authority,
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
    user_transfer_authority: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct WithdrawFromLendingPool {
        instruction: u8,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*source_collateral.key, false),
            AccountMeta::new(*destination_liquidity.key, false),
            AccountMeta::new(*reserve.key, false),
            AccountMeta::new(*reserve_collateral_mint.key, false),
            AccountMeta::new(*reserve_liquidity_supply.key, false),
            AccountMeta::new_readonly(*lending_market.key, false),
            AccountMeta::new_readonly(*lending_market_authority.key, false),
            AccountMeta::new_readonly(*user_transfer_authority.key, true),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: WithdrawFromLendingPool {
            instruction: 5,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            source_collateral,
            destination_liquidity,
            reserve,
            reserve_collateral_mint,
            reserve_liquidity_supply,
            lending_market,
            lending_market_authority,
            user_transfer_authority,
            clock,
        ],
        signed_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn stake<'a>(
    program_id: &Pubkey,
    user_wallet: AccountInfo<'a>,
    user_farming: AccountInfo<'a>,
    user_stake_token: AccountInfo<'a>,
    user_reward_a: AccountInfo<'a>,
    user_reward_b: AccountInfo<'a>,
    farming_pool: AccountInfo<'a>,
    farming_pool_authority: AccountInfo<'a>,
    pool_stake_token: AccountInfo<'a>,
    pool_reward_a: AccountInfo<'a>,
    pool_reward_b: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct Stake {
        instruction: u8,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*user_wallet.key, true),
            AccountMeta::new(*user_farming.key, false),
            AccountMeta::new(*user_stake_token.key, false),
            AccountMeta::new(*user_reward_a.key, false),
            AccountMeta::new(*user_reward_b.key, false),
            AccountMeta::new(*farming_pool.key, false),
            AccountMeta::new_readonly(*farming_pool_authority.key, false),
            AccountMeta::new(*pool_stake_token.key, false),
            AccountMeta::new(*pool_reward_a.key, false),
            AccountMeta::new(*pool_reward_b.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data: Stake {
            instruction: 3,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            user_wallet,
            user_farming,
            user_stake_token,
            user_reward_a,
            user_reward_b,
            farming_pool,
            farming_pool_authority,
            pool_stake_token,
            pool_reward_a,
            pool_reward_b,
            clock,
        ],
        signed_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn unstake<'a>(
    program_id: &Pubkey,
    user_wallet: AccountInfo<'a>,
    user_farming: AccountInfo<'a>,
    user_stake_token: AccountInfo<'a>,
    user_reward_a: AccountInfo<'a>,
    user_reward_b: AccountInfo<'a>,
    farming_pool: AccountInfo<'a>,
    farming_pool_authority: AccountInfo<'a>,
    pool_stake_token: AccountInfo<'a>,
    pool_reward_a: AccountInfo<'a>,
    pool_reward_b: AccountInfo<'a>,
    clock: AccountInfo<'a>,
    amount: u64,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct Unstake {
        instruction: u8,
        amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*user_wallet.key, true),
            AccountMeta::new(*user_farming.key, false),
            AccountMeta::new(*user_stake_token.key, false),
            AccountMeta::new(*user_reward_a.key, false),
            AccountMeta::new(*user_reward_b.key, false),
            AccountMeta::new(*farming_pool.key, false),
            AccountMeta::new_readonly(*farming_pool_authority.key, false),
            AccountMeta::new(*pool_stake_token.key, false),
            AccountMeta::new(*pool_reward_a.key, false),
            AccountMeta::new(*pool_reward_b.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::clock::id(), false),
        ],
        data: Unstake {
            instruction: 4,
            amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            user_wallet,
            user_farming,
            user_stake_token,
            user_reward_a,
            user_reward_b,
            farming_pool,
            farming_pool_authority,
            pool_stake_token,
            pool_reward_a,
            pool_reward_b,
            clock,
        ],
        signed_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn init_farming_user<'a>(
    program_id: &Pubkey,
    user_wallet: AccountInfo<'a>,
    user_farming: AccountInfo<'a>,
    farming_pool: AccountInfo<'a>,
    user_stake_token: AccountInfo<'a>,
    user_reward_a: AccountInfo<'a>,
    user_reward_b: AccountInfo<'a>,
    system_program: AccountInfo<'a>,
    rent_info: AccountInfo<'a>,
    signed_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct Init {
        instruction: u8,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*user_wallet.key, true),
            AccountMeta::new(*user_farming.key, false),
            AccountMeta::new(*farming_pool.key, false),
            AccountMeta::new(*user_stake_token.key, false),
            AccountMeta::new(*user_reward_a.key, false),
            AccountMeta::new(*user_reward_b.key, false),
            AccountMeta::new_readonly(*system_program.key, false),
            AccountMeta::new_readonly(*rent_info.key, false),
        ],
        data: Init { instruction: 1 }.try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            user_wallet,
            user_farming,
            farming_pool,
            user_stake_token,
            user_reward_a,
            user_reward_b,
            rent_info,
        ],
        signed_seeds,
    )
}

pub fn get_staking_program_id() -> Pubkey {
    Pubkey::from_str("3Katmm9dhvLQijAvomteYMo6rfVbY5NaCRNq9ZBqBgr6").unwrap()
}

pub fn find_user_farming_address(
    depositor_authority: &Pubkey,
    farming_pool: &Pubkey,
    user_stake_token_account: &Pubkey,
) -> Pubkey {
    let (user_farming, _) = Pubkey::find_program_address(
        &[
            depositor_authority.as_ref(),
            farming_pool.as_ref(),
            user_stake_token_account.as_ref(),
        ],
        &get_staking_program_id(),
    );
    user_farming
}

pub fn get_real_liquidity_amount(
    reserve: AccountInfo,
    collateral_amount: u64,
) -> Result<u64, ProgramError> {
    let reserve = state::LendingPool::unpack(*reserve.data.borrow())?;

    let total_asset = Decimal::from(reserve.liquidity.available_amount)
        .try_add(reserve.liquidity.borrowed_amount_wads)?;
    let rate = Decimal::from(reserve.shares.mint_total_supply).try_div(total_asset)?;

    Decimal::from(collateral_amount)
        .try_div(rate)?
        .try_floor_u64()
}

pub fn is_deposit_disabled(reserve: AccountInfo) -> Result<bool, ProgramError> {
    let reserve = state::LendingPool::unpack(&reserve.data.borrow())?;
    Ok(reserve.version == DISABLED_DEPOSIT_VALUE)
}

mod state {
    pub const PROGRAM_VERSION: u8 = 1;
    pub const UNINITIALIZED_VERSION: u8 = 0;

    use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
    use solana_program::clock::Slot;
    use solana_program::msg;
    use solana_program::program_error::ProgramError;
    use solana_program::program_option::COption;
    use solana_program::program_pack::{IsInitialized, Pack, Sealed};
    use solana_program::pubkey::Pubkey;
    use spl_token_lending::math::Decimal;

    /// Lending market reserve state
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct LendingPool {
        /// Version of the struct, len = 1
        pub version: u8,
        /// Last slot when supply and rates updated, len = 9
        pub last_update: LastUpdate,
        /// Lending market address, len = 32
        pub lending_market: Pubkey,
        /// Reserve liquidity, len = 193
        pub liquidity: ReserveLiquidity,
        /// Liquidity shares, len = 72
        pub shares: LiquidityShares,
        /// Credit token, len = 72
        pub credit: CreditToken,
        /// InterestRate len = 11
        pub interest_rate_model: InterestRateModel,
        /// interest reverse rate
        pub interest_reverse_rate: u8,
        /// accumulated_interest_reverse: u64
        pub accumulated_interest_reverse: u64,
        // _padding 108
    }

    impl Sealed for LendingPool {}
    impl IsInitialized for LendingPool {
        fn is_initialized(&self) -> bool {
            self.version != UNINITIALIZED_VERSION
        }
    }

    const LENDING_POOL_LEN: usize = 495;
    /// [1, 8, 1, 32, 32, 1, 32, 32, 36, 8, 16, 16, 8, 32, 8, 32,  32, 8, 32, 11,1,8,108]
    impl Pack for LendingPool {
        const LEN: usize = LENDING_POOL_LEN;

        fn pack_into_slice(&self, output: &mut [u8]) {
            let output = array_mut_ref![output, 0, LENDING_POOL_LEN];
            #[allow(clippy::ptr_offset_with_cast)]
            let (
                version,
                last_update_slot,
                last_update_stale,
                lending_market,
                liquidity_mint_pubkey,
                liquidity_mint_decimals,
                liquidity_supply_pubkey,
                liquidity_fee_receiver,
                liquidity_oracle_pubkey,
                liquidity_available_amount,
                liquidity_borrowed_amount_wads,
                liquidity_cumulative_borrow_rate_wads,
                liquidity_market_price,
                share_mint_pubkey,
                share_mint_total_supply,
                share_supply_pubkey,
                credit_mint_pubkey,
                credit_mint_total_supply,
                credit_supply_pubkey,
                interest_model,
                interest_reverse_rate,
                accumulated_interest_reverse,
                _padding,
            ) = mut_array_refs![
                output, 1, 8, 1, 32, 32, 1, 32, 32, 36, 8, 16, 16, 8, 32, 8, 32, 32, 8, 32, 11, 1,
                8, 108
            ];

            // reserve
            *version = self.version.to_le_bytes();
            *last_update_slot = self.last_update.slot.to_le_bytes();
            pack_bool(self.last_update.stale, last_update_stale);
            lending_market.copy_from_slice(self.lending_market.as_ref());

            // liquidity
            liquidity_mint_pubkey.copy_from_slice(self.liquidity.mint_pubkey.as_ref());
            *liquidity_mint_decimals = self.liquidity.mint_decimals.to_le_bytes();
            liquidity_supply_pubkey.copy_from_slice(self.liquidity.supply_pubkey.as_ref());
            liquidity_fee_receiver.copy_from_slice(self.liquidity.fee_receiver.as_ref());
            pack_coption_key(&self.liquidity.oracle_pubkey, liquidity_oracle_pubkey);
            *liquidity_available_amount = self.liquidity.available_amount.to_le_bytes();
            pack_decimal(
                self.liquidity.borrowed_amount_wads,
                liquidity_borrowed_amount_wads,
            );
            pack_decimal(
                self.liquidity.cumulative_borrow_rate_wads,
                liquidity_cumulative_borrow_rate_wads,
            );
            *liquidity_market_price = self.liquidity.market_price.to_le_bytes();

            // share
            share_mint_pubkey.copy_from_slice(self.shares.mint_pubkey.as_ref());
            *share_mint_total_supply = self.shares.mint_total_supply.to_le_bytes();
            share_supply_pubkey.copy_from_slice(self.shares.supply_pubkey.as_ref());

            // credit
            credit_mint_pubkey.copy_from_slice(self.credit.mint_pubkey.as_ref());
            *credit_mint_total_supply = self.credit.mint_total_supply.to_le_bytes();
            credit_supply_pubkey.copy_from_slice(self.credit.supply_pubkey.as_ref());

            // interest model
            self.interest_rate_model.pack(interest_model);

            // interest reverse
            interest_reverse_rate[0] = self.interest_reverse_rate;
            accumulated_interest_reverse
                .copy_from_slice(&self.accumulated_interest_reverse.to_le_bytes());
        }

        fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
            let input = array_ref![input, 0, LENDING_POOL_LEN];
            #[allow(clippy::ptr_offset_with_cast)]
            let (
                version,
                last_update_slot,
                last_update_stale,
                lending_market,
                liquidity_mint_pubkey,
                liquidity_mint_decimals,
                liquidity_supply_pubkey,
                liquidity_fee_receiver,
                liquidity_oracle_pubkey,
                liquidity_available_amount,
                liquidity_borrowed_amount_wads,
                liquidity_cumulative_borrow_rate_wads,
                liquidity_market_price,
                share_mint_pubkey,
                share_mint_total_supply,
                share_supply_pubkey,
                credit_mint_pubkey,
                credit_mint_total_supply,
                credit_supply_pubkey,
                interest_model,
                interest_reverse_rate,
                accumulated_interest_reverse,
                _padding,
            ) = array_refs![
                input, 1, 8, 1, 32, 32, 1, 32, 32, 36, 8, 16, 16, 8, 32, 8, 32, 32, 8, 32, 11, 1,
                8, 108
            ];

            let version = u8::from_le_bytes(*version);
            if version > PROGRAM_VERSION {
                msg!("Reserve version does not match Lending program version");
                return Err(ProgramError::InvalidAccountData);
            }

            let interest_reverse_rate =
                if interest_reverse_rate[0] == 0 || interest_reverse_rate[0] > 50 {
                    10
                } else {
                    interest_reverse_rate[0]
                };

            let ret = Self {
                version,
                last_update: LastUpdate {
                    slot: u64::from_le_bytes(*last_update_slot),
                    stale: unpack_bool(last_update_stale)?,
                },
                lending_market: Pubkey::new_from_array(*lending_market),
                liquidity: ReserveLiquidity {
                    mint_pubkey: Pubkey::new_from_array(*liquidity_mint_pubkey),
                    mint_decimals: u8::from_le_bytes(*liquidity_mint_decimals),
                    supply_pubkey: Pubkey::new_from_array(*liquidity_supply_pubkey),
                    fee_receiver: Pubkey::new_from_array(*liquidity_fee_receiver),
                    oracle_pubkey: unpack_coption_key(liquidity_oracle_pubkey)?,
                    available_amount: u64::from_le_bytes(*liquidity_available_amount),
                    borrowed_amount_wads: unpack_decimal(liquidity_borrowed_amount_wads),
                    cumulative_borrow_rate_wads: unpack_decimal(
                        liquidity_cumulative_borrow_rate_wads,
                    ),
                    market_price: u64::from_le_bytes(*liquidity_market_price),
                },
                shares: LiquidityShares {
                    mint_pubkey: Pubkey::new_from_array(*share_mint_pubkey),
                    mint_total_supply: u64::from_le_bytes(*share_mint_total_supply),
                    supply_pubkey: Pubkey::new_from_array(*share_supply_pubkey),
                },
                credit: CreditToken {
                    mint_pubkey: Pubkey::new_from_array(*credit_mint_pubkey),
                    mint_total_supply: u64::from_le_bytes(*credit_mint_total_supply),
                    supply_pubkey: Pubkey::new_from_array(*credit_supply_pubkey),
                },
                interest_rate_model: InterestRateModel::unpack(interest_model),
                interest_reverse_rate,
                accumulated_interest_reverse: u64::from_le_bytes(*accumulated_interest_reverse),
            };

            Ok(ret)
        }
    }

    /// Reserve liquidity, len 181 [32, 1, 32, 32, 36, 8, 16, 16, 8]
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct ReserveLiquidity {
        /// Reserve liquidity mint address, len 32
        pub mint_pubkey: Pubkey,
        /// Reserve liquidity mint decimals, len 1
        pub mint_decimals: u8,
        /// Reserve liquidity supply address, len 32
        pub supply_pubkey: Pubkey,
        /// Reserve liquidity fee receiver address, len 32
        pub fee_receiver: Pubkey,
        /// Optional reserve liquidity oracle state account, len 4+32
        pub oracle_pubkey: COption<Pubkey>,
        /// Reserve liquidity available, len 8
        pub available_amount: u64,
        /// Reserve liquidity borrowed, len 16
        pub borrowed_amount_wads: Decimal,
        /// Reserve liquidity cumulative borrow rate, len 16
        pub cumulative_borrow_rate_wads: Decimal,
        /// Reserve liquidity market price in quote currency, len 8
        pub market_price: u64,
    }

    /// Credit Token, len 72 = 32+8+32
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct CreditToken {
        /// Reserve collateral mint address
        pub mint_pubkey: Pubkey,
        /// Reserve collateral mint supply, used for exchange rate
        pub mint_total_supply: u64,
        /// Reserve collateral supply address
        pub supply_pubkey: Pubkey,
    }

    /// Liquidity shares, len 72
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct LiquidityShares {
        /// Reserve collateral mint address
        pub mint_pubkey: Pubkey,
        /// Reserve collateral mint supply, used for exchange rate
        pub mint_total_supply: u64,
        /// Reserve collateral supply address
        pub supply_pubkey: Pubkey,
    }

    /// Last update state
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct LastUpdate {
        /// Last slot when updated
        pub slot: Slot,
        /// True when marked stale, false when slot updated
        pub stale: bool,
    }

    /// InterestRateModel Len=11
    #[derive(Clone, Debug, PartialEq)]
    pub struct InterestRateModel {
        pub threshold_1: u8,
        pub threshold_2: u8,
        pub base_1: u8,
        pub factor_1: u16,
        pub base_2: u8,
        pub factor_2: u16,
        pub base_3: u8,
        pub factor_3: u16,
    }

    impl Default for InterestRateModel {
        fn default() -> Self {
            Self {
                threshold_1: 60,
                threshold_2: 90,
                base_1: 0,
                factor_1: 25,
                base_2: 15,
                factor_2: 0,
                base_3: 15,
                factor_3: 1300,
            }
        }
    }

    impl InterestRateModel {
        pub fn pack(&self, dst: &mut [u8; 11]) {
            let (threshold_1, threshold_2, base_1, factor_1, base_2, factor_2, base_3, factor_3) =
                mut_array_refs![dst, 1, 1, 1, 2, 1, 2, 1, 2];

            threshold_1[0] = self.threshold_1;
            threshold_2[0] = self.threshold_2;
            base_1[0] = self.base_1;
            factor_1.copy_from_slice(&self.factor_1.to_le_bytes());
            base_2[0] = self.base_2;
            factor_2.copy_from_slice(&self.factor_2.to_le_bytes());
            base_3[0] = self.base_3;
            factor_3.copy_from_slice(&self.factor_3.to_le_bytes());
        }

        pub fn unpack(src: &[u8; 11]) -> Self {
            if src.eq(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]) {
                return Self::default();
            }

            let (threshold_1, threshold_2, base_1, factor_1, base_2, factor_2, base_3, factor_3) =
                array_refs![src, 1, 1, 1, 2, 1, 2, 1, 2];

            Self {
                threshold_1: threshold_1[0],
                threshold_2: threshold_2[0],
                base_1: base_1[0],
                factor_1: u16::from_le_bytes(*factor_1),
                base_2: base_2[0],
                factor_2: u16::from_le_bytes(*factor_2),
                base_3: base_3[0],
                factor_3: u16::from_le_bytes(*factor_3),
            }
        }
    }

    // Helpers
    fn pack_coption_key(src: &COption<Pubkey>, dst: &mut [u8; 36]) {
        let (tag, body) = mut_array_refs![dst, 4, 32];
        match src {
            COption::Some(key) => {
                *tag = [1, 0, 0, 0];
                body.copy_from_slice(key.as_ref());
            }
            COption::None => {
                *tag = [0; 4];
            }
        }
    }

    fn unpack_coption_key(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
        let (tag, body) = array_refs![src, 4, 32];
        match *tag {
            [0, 0, 0, 0] => Ok(COption::None),
            [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    fn pack_decimal(decimal: Decimal, dst: &mut [u8; 16]) {
        *dst = decimal
            .to_scaled_val()
            .expect("Decimal cannot be packed")
            .to_le_bytes();
    }

    fn unpack_decimal(src: &[u8; 16]) -> Decimal {
        Decimal::from_scaled_val(u128::from_le_bytes(*src))
    }

    fn pack_bool(boolean: bool, dst: &mut [u8; 1]) {
        *dst = (boolean as u8).to_le_bytes()
    }

    fn unpack_bool(src: &[u8; 1]) -> Result<bool, ProgramError> {
        match u8::from_le_bytes(*src) {
            0 => Ok(false),
            1 => Ok(true),
            _ => {
                msg!("Boolean cannot be unpacked");
                Err(ProgramError::InvalidAccountData)
            }
        }
    }
}
