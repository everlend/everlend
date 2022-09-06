use everlend_utils::{assert_account_key, cpi, AccountLoader};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_program,
    sysvar::{Sysvar, SysvarId},
};

use crate::{
    find_pool_borrow_authority_program_address,
    state::{InitPoolBorrowAuthorityParams, Pool, PoolBorrowAuthority, PoolMarket},
};

/// Instruction context
pub struct CreatePoolBorrowAuthorityContext<'a, 'b> {
    borrow_authority: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    pool_market: &'a AccountInfo<'b>,
    pool_borrow_authority: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> CreatePoolBorrowAuthorityContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<CreatePoolBorrowAuthorityContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let pool_market = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let pool_borrow_authority = AccountLoader::next_uninitialized(account_info_iter)?;
        let borrow_authority = AccountLoader::next_unchecked(account_info_iter)?; // Can be any account
        let manager = AccountLoader::next_signer(account_info_iter)?;
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

        Ok(CreatePoolBorrowAuthorityContext {
            borrow_authority,
            manager,
            pool,
            pool_borrow_authority,
            rent,
            pool_market,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey, share_allowed: u16) -> ProgramResult {
        // Check manager
        {
            let pool_market = PoolMarket::unpack(&self.pool_market.data.borrow())?;
            assert_account_key(self.manager, &pool_market.manager)?;

            // Get pool state
            let pool = Pool::unpack(&self.pool.data.borrow())?;
            assert_account_key(self.pool_market, &pool.pool_market)?;
        }

        let rent = &Rent::from_account_info(self.rent)?;

        {
            // Create pool borrow authority account
            let (pool_borrow_authority_pubkey, bump_seed) =
                find_pool_borrow_authority_program_address(
                    program_id,
                    self.pool.key,
                    self.borrow_authority.key,
                );

            assert_account_key(self.pool_borrow_authority, &pool_borrow_authority_pubkey)?;

            let signers_seeds = &[
                &self.pool.key.to_bytes()[..32],
                &self.borrow_authority.key.to_bytes()[..32],
                &[bump_seed],
            ];

            cpi::system::create_account::<PoolBorrowAuthority>(
                program_id,
                self.manager.clone(),
                self.pool_borrow_authority.clone(),
                &[signers_seeds],
                rent,
            )?;
        }

        let pool_borrow_authority = PoolBorrowAuthority::init(InitPoolBorrowAuthorityParams {
            pool: *self.pool.key,
            borrow_authority: *self.borrow_authority.key,
            share_allowed,
        });

        PoolBorrowAuthority::pack(
            pool_borrow_authority,
            *self.pool_borrow_authority.data.borrow_mut(),
        )?;

        Ok(())
    }
}
