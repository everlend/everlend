use crate::state::Depositor;
use crate::{TransitPDA, ALLOWED_TRANSIT_SEEDS};
use everlend_utils::{assert_account_key, cpi, find_program_address, AccountLoader, PDA};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey, rent::Rent, system_program, sysvar::Sysvar,
    sysvar::SysvarId,
};
use std::{iter::Enumerate, slice::Iter};

/// Instruction context
pub struct CreateTransitContext<'a, 'b> {
    depositor: &'a AccountInfo<'b>,
    transit: &'a AccountInfo<'b>,
    mint: &'a AccountInfo<'b>,
    depositor_authority: &'a AccountInfo<'b>,
    from: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> CreateTransitContext<'a, 'b> {
    /// New CreateTransit instruction context
    pub fn new(
        program_id: &Pubkey,
        account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
    ) -> Result<CreateTransitContext<'a, 'b>, ProgramError> {
        let depositor = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let depositor_authority = AccountLoader::next_unchecked(account_info_iter)?;

        // Uninitialized token account
        let transit = AccountLoader::next_uninitialized(account_info_iter)?;
        let mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;

        let from = AccountLoader::next_signer(account_info_iter)?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;

        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;
        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;

        Ok(CreateTransitContext {
            depositor,
            transit,
            mint,
            depositor_authority,
            from,
            rent,
        })
    }

    /// Process CreateTransit instruction
    pub fn process(
        &self,
        program_id: &Pubkey,
        _account_info_iter: &mut Enumerate<Iter<'a, AccountInfo<'b>>>,
        seed: String,
    ) -> ProgramResult {
        // Check seed if it's allowed
        if !ALLOWED_TRANSIT_SEEDS.contains(&seed.as_str()) {
            return Err(ProgramError::InvalidArgument);
        }

        // Check depositor initialized
        Depositor::unpack(&self.depositor.data.borrow())?;

        // Check depositor authority
        {
            let (depositor_authority_pubkey, _) =
                find_program_address(program_id, self.depositor.key);
            assert_account_key(self.depositor_authority, &depositor_authority_pubkey)
        }?;

        // Create transit account for SPL program
        let seeds = {
            let pda = TransitPDA {
                depositor: *self.depositor.key,
                mint: *self.mint.key,
                seed: &seed,
            };

            let (transit_pubkey, bump) = pda.find_address(program_id);
            assert_account_key(self.transit, &transit_pubkey)?;

            pda.get_signing_seeds(bump)
        };

        {
            let rent = &Rent::from_account_info(self.rent)?;

            cpi::system::create_account::<spl_token::state::Account>(
                &spl_token::id(),
                self.from.clone(),
                self.transit.clone(),
                &[&seeds.as_seeds_slice()],
                rent,
            )?;
        }

        // Initialize transit token account for spl token
        cpi::spl_token::initialize_account(
            self.transit.clone(),
            self.mint.clone(),
            self.depositor_authority.clone(),
            self.rent.clone(),
        )?;

        Ok(())
    }
}
