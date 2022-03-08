use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Total withdraw request for fixed state
pub const TOTAL_WITHDRAW_REQUEST: usize = 20;

/// Rebalancing
#[repr(C)]
#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct WithdrawalRequests {
    /// Account type - Rebalancing
    pub account_type: AccountType,

    /// Pool
    pub pool: Pubkey,

    /// Mint
    pub mint: Pubkey,

    /// Withdraw request id
    pub last_request_id: u64,

    /// Last processed request id
    pub last_processed_request_id: u64,

    /// Total requests amount
    pub liquidity_supply: u64,
}

/// RebalancingStep
#[repr(C)]
#[derive(Debug, Clone, Copy, BorshDeserialize, BorshSerialize, BorshSchema, PartialEq, Default)]
pub struct WithdrawalRequest {
    /// Rent payer
    pub rent_payer: Pubkey,

    /// Withdraw source
    pub source: Pubkey,

    /// Withdraw destination
    pub destination: Pubkey,

    /// Withdraw liquidity amount
    pub liquidity_amount: u64,

    /// Withdraw collateral amount
    pub collateral_amount: u64,
}

/// Initialize a Rebalancing params
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
}

impl Sealed for WithdrawalRequests {}
impl Pack for WithdrawalRequests {
    // 1 + 32 + 32 + 8 + 8 +8
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

impl Sealed for WithdrawalRequest {}
impl Pack for WithdrawalRequest {
    // 32 + 32 + 8 + 8
    const LEN: usize = 112;

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
        self.collateral_amount != 0
    }
}
