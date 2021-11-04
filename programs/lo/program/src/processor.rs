//! Program state processor.

use crate::{
    error::LiquidityOracleError,
    instruction::LiquidityOracleInstruction,
    state::{InitLiquidityOracleParams, LiquidityOracle},
    utils::*,
};

use crate::state::AccountType;
use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Process `InitLiquidityOracle` instruction.
    pub fn init_liquidity_oracle(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        // Check signer
        assert_signer(authority_info)?;

        // Check liquidity oracle owner
        assert_owned_by(liquidity_oracle_info, program_id)?;

        // Get state
        let mut liquidity_oracle =
            LiquidityOracle::unpack_unchecked(&liquidity_oracle_info.data.borrow())?;

        //Check init once
        if liquidity_oracle.account_type != AccountType::Uninitialized {
            return Err(LiquidityOracleError::AlreadyInitialized.into());
        }

        // Initialize
        liquidity_oracle.init(InitLiquidityOracleParams {
            authority: *authority_info.key,
        });

        // Save state
        LiquidityOracle::pack(liquidity_oracle, *liquidity_oracle_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process `UpdateLiquidityOracleAuthority` instruction.
    pub fn update_liquidity_oracle_authority(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        value: [u8; 32],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        // Check signer
        assert_signer(authority_info)?;

        // Check liquidity oracle owner
        assert_owned_by(liquidity_oracle_info, program_id)?;

        // Get state
        let mut liquidity_oracle = LiquidityOracle::unpack(&liquidity_oracle_info.data.borrow())?;

        // Check liquidity oracle authority
        if liquidity_oracle.authority != *authority_info.key {
            return Err(ProgramError::InvalidArgument);
        }

        // Update
        liquidity_oracle.update(Pubkey::new(&value));

        // Save state
        LiquidityOracle::pack(liquidity_oracle, *liquidity_oracle_info.data.borrow_mut())?;

        Ok(())
    }

    /// Instruction processing router.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = LiquidityOracleInstruction::try_from_slice(input)?;

        match instruction {
            LiquidityOracleInstruction::InitLiquidityOracle => {
                msg!("LiquidityOracleInstruction: InitLiquidityOracle");
                Self::init_liquidity_oracle(program_id, accounts)
            }
            LiquidityOracleInstruction::UpdateLiquidityOracleAuthority { value } => {
                msg!("LiquidityOracleInstruction: UpdateLiquidityOracleAuthority");
                Self::update_liquidity_oracle_authority(program_id, accounts, value)
            }
        }
    }
}
