use super::{AccountType, AccountVersion};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::{EverlendError, Uninitialized};
use solana_program::{
    clock::Slot,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// How long after the request, you can execute a withdraw
pub const WITHDRAW_DELAY: Slot = 200;

/// Actual version of withdrawal requests struct
pub const ACTUAL_VERSION: AccountVersion = AccountVersion::V0;

/// Withdrawal requests
#[repr(C)]
#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct WithdrawalRequests {
    /// Account type - WithdrawalRequests
    pub account_type: AccountType,

    /// Account version
    pub account_version: AccountVersion,

    /// Pool
    pub pool: Pubkey,

    /// Mint
    pub mint: Pubkey,

    /// Total requests amount
    pub liquidity_supply: u64,
}

/// Initialize a withdrawal requests params
pub struct InitWithdrawalRequestsParams {
    /// Pool
    pub pool: Pubkey,
    /// Mint
    pub mint: Pubkey,
}

impl WithdrawalRequests {
    /// Initialize a withdrawal requests
    pub fn init(params: InitWithdrawalRequestsParams) -> WithdrawalRequests {
        WithdrawalRequests {
            account_type: AccountType::WithdrawRequests,
            account_version: ACTUAL_VERSION,
            pool: params.pool,
            mint: params.mint,
            liquidity_supply: 0,
        }
    }

    /// Add new withdrawal request
    pub fn add(&mut self, liquidity_amount: u64) -> ProgramResult {
        self.liquidity_supply = self
            .liquidity_supply
            .checked_add(liquidity_amount)
            .ok_or(EverlendError::MathOverflow)?;

        Ok(())
    }

    /// Remove first withdrawal request
    pub fn process(&mut self, liquidity_amount: u64) -> ProgramResult {
        self.liquidity_supply = self
            .liquidity_supply
            .checked_sub(liquidity_amount)
            .ok_or(EverlendError::MathOverflow)?;

        Ok(())
    }
}

impl Sealed for WithdrawalRequests {}
impl Pack for WithdrawalRequests {
    // 1 + 1 + 32 + 32 + 8
    const LEN: usize = 74;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut src_mut = src;
        Self::deserialize(&mut src_mut).map_err(|err| {
            msg!("Failed to deserialize");
            msg!(&err.to_string());
            ProgramError::InvalidAccountData
        })
    }
}

impl IsInitialized for WithdrawalRequests {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::WithdrawRequests && self.account_version == ACTUAL_VERSION
    }
}

impl Uninitialized for WithdrawalRequests {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}

/// Withdrawal request
#[repr(C)]
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct WithdrawalRequest {
    /// Account type - WithdrawalRequest
    pub account_type: AccountType,

    /// Pool
    pub pool: Pubkey,

    /// From account
    pub from: Pubkey,

    /// Withdraw source
    pub source: Pubkey,

    /// Withdraw destination
    pub destination: Pubkey,

    /// Withdraw liquidity amount
    pub liquidity_amount: u64,

    /// Withdraw collateral amount
    pub collateral_amount: u64,

    /// Slot after which you can withdraw
    pub ticket: Slot,
}

impl WithdrawalRequest {
    /// Initialize a withdrawal request
    pub fn init(params: InitWithdrawalRequestParams) -> WithdrawalRequest {
        WithdrawalRequest {
            account_type: AccountType::WithdrawRequest,
            pool: params.pool,
            from: params.from,
            source: params.source,
            destination: params.destination,
            liquidity_amount: params.liquidity_amount,
            collateral_amount: params.collateral_amount,
            ticket: params.ticket,
        }
    }
}

/// Initialize a withdrawal request params
pub struct InitWithdrawalRequestParams {
    /// Pool
    pub pool: Pubkey,
    /// From account
    pub from: Pubkey,
    /// Withdraw source
    pub source: Pubkey,
    /// Withdraw destination
    pub destination: Pubkey,
    /// Withdraw liquidity amount
    pub liquidity_amount: u64,
    /// Withdraw collateral amount
    pub collateral_amount: u64,
    /// Slot after which you can withdraw
    pub ticket: Slot,
}

impl Sealed for WithdrawalRequest {}
impl Pack for WithdrawalRequest {
    // 1 + 32 + 32 + 32 + 32 + 8 + 8 + 8
    const LEN: usize = 153;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let mut slice = dst;
        self.serialize(&mut slice).unwrap()
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Self::try_from_slice(src).map_err(|_| {
            msg!("Failed to deserialize");
            msg!("Actual LEN: {}", std::mem::size_of::<WithdrawalRequest>());
            ProgramError::InvalidAccountData
        })
    }
}
impl IsInitialized for WithdrawalRequest {
    fn is_initialized(&self) -> bool {
        self.account_type == AccountType::WithdrawRequest
    }
}

impl Uninitialized for WithdrawalRequest {
    fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::default()
    }
}
