//! Program state processor

use crate::{
    find_transit_program_address,
    instruction::DepositorInstruction,
    state::Depositor,
    utils::{
        assert_account_key, assert_owned_by, assert_rent_exempt, assert_uninitialized,
        check_deposit, create_account, spl_initialize_account, ulp_borrow,
    },
};
use borsh::BorshDeserialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
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
    /// Process Init instruction
    pub fn init(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let depositor_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;

        assert_rent_exempt(rent, depositor_info)?;
        assert_owned_by(depositor_info, program_id)?;

        // Get depositor state
        let mut depositor = Depositor::unpack_unchecked(&depositor_info.data.borrow())?;
        assert_uninitialized(&depositor)?;

        depositor.init();

        Depositor::pack(depositor, *depositor_info.data.borrow_mut())?;

        Ok(())
    }

    /// Process CreateTransit instruction
    pub fn create_transit(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let depositor_info = next_account_info(account_info_iter)?;
        let transit_info = next_account_info(account_info_iter)?;
        let token_mint_info = next_account_info(account_info_iter)?;
        let depositor_authority_info = next_account_info(account_info_iter)?;
        let from_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;
        let _system_program_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        assert_owned_by(depositor_info, program_id)?;

        // Get depositor state
        // Check initialized
        Depositor::unpack(&depositor_info.data.borrow())?;

        // Create transit account for SPL program
        let (transit_pubkey, bump_seed) =
            find_transit_program_address(program_id, depositor_info.key, token_mint_info.key);
        assert_account_key(transit_info, &transit_pubkey)?;

        let signers_seeds = &[
            &depositor_info.key.to_bytes()[..32],
            &token_mint_info.key.to_bytes()[..32],
            &[bump_seed],
        ];

        create_account::<spl_token::state::Account>(
            &spl_token::id(),
            from_info.clone(),
            transit_info.clone(),
            &[signers_seeds],
            rent,
        )?;

        // Initialize transit token account for spl token
        spl_initialize_account(
            transit_info.clone(),
            token_mint_info.clone(),
            depositor_authority_info.clone(),
            rent_info.clone(),
        )?;

        Ok(())
    }

    /// Process Deposit instruction
    pub fn deposit(program_id: &Pubkey, amount: u64, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let depositor_info = next_account_info(account_info_iter)?;
        let pool_market_info = next_account_info(account_info_iter)?;
        let pool_info = next_account_info(account_info_iter)?;
        let pool_borrow_authority_info = next_account_info(account_info_iter)?;
        let pool_market_authority_info = next_account_info(account_info_iter)?;
        let pool_token_account_info = next_account_info(account_info_iter)?;
        let transit_info = next_account_info(account_info_iter)?;
        let token_mint_info = next_account_info(account_info_iter)?;
        let rebalancer_info = next_account_info(account_info_iter)?;
        let _everlend_ulp_info = next_account_info(account_info_iter)?;
        let instructions_info = next_account_info(account_info_iter)?;
        let _token_program_info = next_account_info(account_info_iter)?;

        if !rebalancer_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Borrow from ULP to transit account
        ulp_borrow(
            pool_market_info.clone(),
            pool_info.clone(),
            pool_borrow_authority_info.clone(),
            pool_market_authority_info.clone(),
            transit_info.clone(),
            pool_token_account_info.clone(),
            rebalancer_info.clone(),
            amount,
            &[],
        )?;

        // check_deposit(instructions_info, amount)?;

        // ...

        Ok(())
    }

    /// Instruction processing router
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = DepositorInstruction::try_from_slice(input)?;

        match instruction {
            DepositorInstruction::Init => {
                msg!("DepositorInstruction: Init");
                Self::init(program_id, accounts)
            }

            DepositorInstruction::CreateTransit => {
                msg!("DepositorInstruction: CreateTransit");
                Self::create_transit(program_id, accounts)
            }

            DepositorInstruction::Deposit { amount } => {
                msg!("DepositorInstruction: Deposit");
                Self::deposit(program_id, amount, accounts)
            }
        }
    }
}
