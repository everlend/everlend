use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::EverlendError;
use solana_program::{
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Withdrawal requests
#[repr(C)]
#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct WithdrawalRequests {
    /// Account type - WithdrawalRequests
    pub account_type: AccountType,

    /// Pool
    pub pool: Pubkey,

    /// Mint
    pub mint: Pubkey,

    /// Next request index
    pub next_ticket: u64,

    /// Next process index
    pub next_process_ticket: u64,

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
    pub fn init(&mut self, params: InitWithdrawalRequestsParams) {
        self.account_type = AccountType::WithdrawRequests;
        self.pool = params.pool;
        self.mint = params.mint;
    }

    /// Add new withdrawal request
    pub fn add(&mut self, liquidity_amount: u64) -> ProgramResult {
        self.liquidity_supply = self
            .liquidity_supply
            .checked_add(liquidity_amount)
            .ok_or(EverlendError::MathOverflow)?;

        self.next_ticket += 1;

        Ok(())
    }

    /// Remove first withdrawal request
    pub fn process(&mut self, liquidity_amount: u64) -> ProgramResult {
        self.liquidity_supply = self
            .liquidity_supply
            .checked_sub(liquidity_amount)
            .ok_or(EverlendError::MathOverflow)?;

        self.next_process_ticket += 1;

        Ok(())
    }
}

impl Sealed for WithdrawalRequests {}
impl Pack for WithdrawalRequests {
    // 1 + 32 + 32 + 8 + 8 + 8
    const LEN: usize = 89;

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
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::WithdrawRequests
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

    /// Index in the requests queue
    pub ticket: u64,
}

impl WithdrawalRequest {
    /// Initialize a withdrawal request
    pub fn init(&mut self, params: InitWithdrawalRequestParams) {
        self.account_type = AccountType::WithdrawRequest;
        self.pool = params.pool;
        self.from = params.from;
        self.source = params.source;
        self.destination = params.destination;
        self.liquidity_amount = params.liquidity_amount;
        self.collateral_amount = params.collateral_amount;
        self.ticket = params.ticket;
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
    /// Index in the requests queue
    pub ticket: u64,
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
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::WithdrawRequest
    }
}
