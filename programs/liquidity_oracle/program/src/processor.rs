//! Program state processor.

use crate::{
    error::LiquidityOracleError,
    find_liquidity_oracle_currency_distribution_program_address,
    instruction::LiquidityOracleInstruction,
    state::{CurrencyDistribution, DistributionArray, InitLiquidityOracleParams, LiquidityOracle},
    utils::*,
};

use crate::state::AccountType;
use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
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
        authority: Pubkey,
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
        liquidity_oracle.update(authority);

        // Save state
        LiquidityOracle::pack(liquidity_oracle, *liquidity_oracle_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process `CreateCurrencyDistribution` instruction.
    pub fn create_currency_distribution(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        currency: String,
        value: DistributionArray,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let currency_distribution_account = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;

        // Check signer
        assert_signer(authority_info)?;

        // Check liquidity oracle owner
        assert_owned_by(liquidity_oracle_info, program_id)?;

        // Check currency distribution account
        let (currency_distribution_pubkey, bump_seed) =
            find_liquidity_oracle_currency_distribution_program_address(
                program_id,
                liquidity_oracle_info.key,
                &currency,
            );
        if currency_distribution_pubkey != *currency_distribution_account.key {
            msg!("Currency distribution provided does not match generated currency distribution");
            return Err(ProgramError::InvalidArgument);
        }

        let mut distribution = CurrencyDistribution::default();

        // Init account type
        distribution.init();
        if currency_distribution_account.data.borrow().len() > 0 {
            distribution = CurrencyDistribution::unpack_unchecked(
                &currency_distribution_account.data.borrow(),
            )?;
            assert_uninitialized(&distribution)?;
        }

        let signers_seeds = &[
            &liquidity_oracle_info.key.to_bytes()[..32],
            currency.as_bytes(),
            &[bump_seed],
        ];

        // Create distribution storage account
        create_account::<CurrencyDistribution>(
            program_id,
            authority_info.clone(),
            currency_distribution_account.clone(),
            &[signers_seeds],
            rent,
        )?;

        distribution.update(clock.slot, value);

        CurrencyDistribution::pack(
            distribution,
            *currency_distribution_account.data.borrow_mut(),
        )?;

        Ok(())
    }

    /// Process `UpdateCurrencyDistribution` instruction.
    pub fn update_currency_distribution(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        currency: String,
        value: DistributionArray,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let currency_distribution_account = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        // Check signer
        assert_signer(authority_info)?;

        // Check liquidity oracle owner
        assert_owned_by(liquidity_oracle_info, program_id)?;

        let (currency_distribution_pubkey, _) =
            find_liquidity_oracle_currency_distribution_program_address(
                program_id,
                liquidity_oracle_info.key,
                &currency,
            );
        if currency_distribution_pubkey != *currency_distribution_account.key {
            msg!("Currency distribution provided does not match generated currency distribution");
            return Err(ProgramError::InvalidArgument);
        }

        let mut distribution =
            CurrencyDistribution::unpack_unchecked(&currency_distribution_account.data.borrow())?;

        assert_initialized(&distribution)?;

        distribution.update(clock.slot, value);

        CurrencyDistribution::pack(
            distribution,
            *currency_distribution_account.data.borrow_mut(),
        )?;

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
            LiquidityOracleInstruction::UpdateLiquidityOracleAuthority { authority } => {
                msg!("LiquidityOracleInstruction: UpdateLiquidityOracleAuthority");
                Self::update_liquidity_oracle_authority(program_id, accounts, authority)
            }
            LiquidityOracleInstruction::CreateCurrencyDistribution { currency, value } => {
                msg!("LiquidityOracleInstruction: CreateCurrencyDistribution");
                Self::create_currency_distribution(program_id, accounts, currency, value)
            }
            LiquidityOracleInstruction::UpdateCurrencyDistribution { currency, value } => {
                msg!("LiquidityOracleInstruction: UpdateCurrencyDistribution");
                Self::update_currency_distribution(program_id, accounts, currency, value)
            }
        }
    }
}
