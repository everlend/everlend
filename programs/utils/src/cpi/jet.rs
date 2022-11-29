use borsh::{BorshDeserialize, BorshSerialize};
use jet_proto_math::Number;
use solana_program::account_info::AccountInfo;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// `global:deposit` anchor program instruction
const DEPOSIT_INSTRUCTION: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];
/// `global:withdraw` anchor program instruction
const REDEEM_INSTRUCTION: [u8; 8] = [183, 18, 70, 156, 148, 109, 161, 34];
/// Enable deposit flag value
const ENABLE_DEPOSIT_FLAG_VALUE: u64 = 2;

#[derive(BorshSerialize, Debug, PartialEq)]
#[repr(u8)]
pub enum ChangeKind {
    SetTo,
    ShiftBy,
}

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

impl MarginPool {
    pub fn convert_amount(&self, amount: u64) -> u64 {
        let exchange_rate = self.deposit_note_exchange_rate();
        let amount = (Number::from(amount) * exchange_rate).as_u64(0);

        return amount;
    }

    fn deposit_note_exchange_rate(&self) -> Number {
        let deposit_notes = std::cmp::max(1, self.deposit_notes);
        let total_value = std::cmp::max(Number::ONE, self.total_value());
        (total_value - Number::from_bits(self.uncollected_fees)) / Number::from(deposit_notes)
    }

    fn total_value(&self) -> Number {
        Number::from_bits(self.borrowed_tokens) + Number::from(self.deposit_tokens)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    program_id: &Pubkey,
    margin_pool: AccountInfo<'a>,
    vault: AccountInfo<'a>,
    deposit_note_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    source_liquidity: AccountInfo<'a>,
    destination_collateral: AccountInfo<'a>,
    liquidity_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct DepositToLendingPool {
        instruction: [u8; 8],
        change_kind: ChangeKind,
        liquidity_amount: u64,
    }

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*margin_pool.key, false),
            AccountMeta::new(*vault.key, false),
            AccountMeta::new(*deposit_note_mint.key, false),
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new(*source_liquidity.key, false),
            AccountMeta::new(*destination_collateral.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: DepositToLendingPool {
            instruction: DEPOSIT_INSTRUCTION,
            change_kind: ChangeKind::ShiftBy,
            liquidity_amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            margin_pool,
            vault,
            deposit_note_mint,
            authority,
            source_liquidity,
            destination_collateral,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn redeem<'a>(
    program_id: &Pubkey,
    margin_pool: AccountInfo<'a>,
    vault: AccountInfo<'a>,
    deposit_note_mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    source_collateral: AccountInfo<'a>,
    destination_liquidity: AccountInfo<'a>,
    collateral_amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    #[derive(Debug, PartialEq, BorshSerialize)]
    pub struct WithdrawFromLendingPool {
        instruction: [u8; 8],
        change_kind: ChangeKind,
        liquidity_amount: u64,
    }

    let mp = MarginPool::try_from_slice(*margin_pool.data.borrow())?;
    let liquidity_amount = mp.convert_amount(collateral_amount);

    let ix = Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new_readonly(*authority.key, true),
            AccountMeta::new(*margin_pool.key, false),
            AccountMeta::new(*vault.key, false),
            AccountMeta::new(*deposit_note_mint.key, false),
            AccountMeta::new(*source_collateral.key, false),
            AccountMeta::new(*destination_liquidity.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: WithdrawFromLendingPool {
            instruction: REDEEM_INSTRUCTION,
            change_kind: ChangeKind::ShiftBy,
            liquidity_amount,
        }
        .try_to_vec()?,
    };

    invoke_signed(
        &ix,
        &[
            authority,
            margin_pool,
            vault,
            deposit_note_mint,
            source_collateral,
            destination_liquidity,
        ],
        signers_seeds,
    )
}

pub fn get_real_liquidity_amount(
    margin_pool: AccountInfo,
    collateral_amount: u64,
) -> Result<u64, ProgramError> {
    let mp = MarginPool::try_from_slice(*margin_pool.data.borrow())?;

    Ok(mp.convert_amount(collateral_amount))
}

pub fn is_deposit_disabled(margin_pool: AccountInfo) -> Result<bool, ProgramError> {
    let mp: MarginPool = MarginPool::try_from_slice(*margin_pool.data.borrow())?;

    Ok(mp.config.flags != ENABLE_DEPOSIT_FLAG_VALUE)
}
