use super::AccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use everlend_utils::EverlendError;
use solana_program::{
    clock::Slot,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Withdrawal requests deprecated
#[repr(C)]
#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct WithdrawalRequestsDeprecated {
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
pub struct InitWithdrawalRequestsDeprecatedParams {
    /// Pool
    pub pool: Pubkey,
    /// Mint
    pub mint: Pubkey,
}

impl WithdrawalRequestsDeprecated {
    /// Initialize a withdrawal requests
    pub fn init(&mut self, params: InitWithdrawalRequestsDeprecatedParams) {
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

impl Sealed for WithdrawalRequestsDeprecated {}
impl Pack for WithdrawalRequestsDeprecated {
    // 1 + 32 + 32 + 8
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

impl IsInitialized for WithdrawalRequestsDeprecated {
    fn is_initialized(&self) -> bool {
        self.account_type != AccountType::Uninitialized
            && self.account_type == AccountType::WithdrawRequests
    }
}
