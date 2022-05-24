//! Program state processor.

use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use everlend_utils::{
    assert_account_key, assert_owned_by, assert_signer, assert_uninitialized, cpi,
};

use crate::{
    find_liquidity_oracle_token_distribution_program_address,
    instruction::LiquidityOracleInstruction,
    state::{DistributionArray, InitLiquidityOracleParams, LiquidityOracle, TokenDistribution},
};

/// Program state handler.
pub struct Processor {}

impl Processor {
    /// Process `InitLiquidityOracle` instruction.
    pub fn init_liquidity_oracle(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        assert_signer(authority_info)?;

        // Check programs
        assert_owned_by(liquidity_oracle_info, program_id)?;

        let mut liquidity_oracle =
            LiquidityOracle::unpack_unchecked(&liquidity_oracle_info.data.borrow())?;
        assert_uninitialized(&liquidity_oracle)?;

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
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let update_authority = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;

        assert_signer(authority_info)?;

        // Check programs
        assert_owned_by(liquidity_oracle_info, program_id)?;

        let mut liquidity_oracle = LiquidityOracle::unpack(&liquidity_oracle_info.data.borrow())?;

        // Check current authority
        assert_account_key(authority_info, &liquidity_oracle.authority)?;

        // Update to new authority
        liquidity_oracle.update(*update_authority.key);

        LiquidityOracle::pack(liquidity_oracle, *liquidity_oracle_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process `CreateTokenDistribution` instruction.
    pub fn create_token_distribution(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        value: DistributionArray,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let token_mint = next_account_info(account_info_iter)?;
        let token_distribution_account = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;

        assert_signer(authority_info)?;

        // Check programs
        assert_owned_by(liquidity_oracle_info, program_id)?;

        let liquidity_oracle = LiquidityOracle::unpack(&liquidity_oracle_info.data.borrow())?;

        // Check authotiry
        assert_account_key(authority_info, &liquidity_oracle.authority)?;

        // Check token distribution account
        let (token_distribution_pubkey, bump_seed) =
            find_liquidity_oracle_token_distribution_program_address(
                program_id,
                liquidity_oracle_info.key,
                token_mint.key,
            );
        // msg!("Token distribution provided does not match generated token distribution");
        assert_account_key(token_distribution_account, &token_distribution_pubkey)?;

        let mut distribution = TokenDistribution::default();

        // Init account type
        distribution.init();

        // TODO: If this is a creation instruction, then it is supposed to mean only the creation,
        // and we can safely assume that the account does not exist.

        if token_distribution_account.data.borrow().len() > 0 {
            distribution =
                TokenDistribution::unpack_unchecked(&token_distribution_account.data.borrow())?;
            assert_uninitialized(&distribution)?;
        }

        let signers_seeds = &[
            &liquidity_oracle_info.key.to_bytes()[..32],
            &token_mint.key.to_bytes()[..32],
            &[bump_seed],
        ];

        // Create distribution storage account
        cpi::system::create_account::<TokenDistribution>(
            program_id,
            authority_info.clone(),
            token_distribution_account.clone(),
            &[signers_seeds],
            rent,
        )?;

        distribution.update(clock.slot, value);

        TokenDistribution::pack(distribution, *token_distribution_account.data.borrow_mut())?;

        Ok(())
    }

    /// Process `UpdateTokenDistribution` instruction.
    pub fn update_token_distribution(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        value: DistributionArray,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let liquidity_oracle_info = next_account_info(account_info_iter)?;
        let token_mint = next_account_info(account_info_iter)?;
        let token_distribution_account = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let clock_info = next_account_info(account_info_iter)?;
        let clock = Clock::from_account_info(clock_info)?;

        assert_signer(authority_info)?;

        // Check programs
        assert_owned_by(liquidity_oracle_info, program_id)?;

        // Get state
        let liquidity_oracle = LiquidityOracle::unpack(&liquidity_oracle_info.data.borrow())?;

        // Check authotiry
        assert_account_key(authority_info, &liquidity_oracle.authority)?;

        // Check token distribution
        let (token_distribution_pubkey, _) =
            find_liquidity_oracle_token_distribution_program_address(
                program_id,
                liquidity_oracle_info.key,
                token_mint.key,
            );
        // msg!("Token distribution provided does not match generated token distribution");
        assert_account_key(token_distribution_account, &token_distribution_pubkey)?;

        let mut distribution =
            TokenDistribution::unpack(&token_distribution_account.data.borrow())?;

        distribution.update(clock.slot, value);

        TokenDistribution::pack(distribution, *token_distribution_account.data.borrow_mut())?;

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
            LiquidityOracleInstruction::UpdateLiquidityOracleAuthority => {
                msg!("LiquidityOracleInstruction: UpdateLiquidityOracleAuthority");
                Self::update_liquidity_oracle_authority(program_id, accounts)
            }
            LiquidityOracleInstruction::CreateTokenDistribution { value } => {
                msg!("LiquidityOracleInstruction: CreateTokenDistribution");
                Self::create_token_distribution(program_id, accounts, value)
            }
            LiquidityOracleInstruction::UpdateTokenDistribution { value } => {
                msg!("LiquidityOracleInstruction: UpdateTokenDistribution");
                Self::update_token_distribution(program_id, accounts, value)
            }
        }
    }
}
