use crate::{
    find_transit_program_address, find_transit_sol_unwrap_address,
    state::{Pool, WithdrawalRequest, WithdrawalRequests},
};
use everlend_utils::{assert_account_key, cpi, find_program_address, AccountLoader, EverlendError};
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_program,
    sysvar::{Sysvar, SysvarId},
};

struct NativeSolContext<'a, 'b> {
    unwrap_sol: &'a AccountInfo<'b>,
    signer: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
    token_mint: &'a AccountInfo<'b>,
}
/// Instruction context
pub struct WithdrawContext<'a, 'b> {
    pool_market: &'a AccountInfo<'b>,
    pool_market_authority: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    pool_mint: &'a AccountInfo<'b>,
    withdrawal_requests: &'a AccountInfo<'b>,
    withdrawal_request: &'a AccountInfo<'b>,
    destination: &'a AccountInfo<'b>,
    token_account: &'a AccountInfo<'b>,
    collateral_transit: &'a AccountInfo<'b>,
    from: &'a AccountInfo<'b>,
    clock: &'a AccountInfo<'b>,
    native_sol_context: Option<NativeSolContext<'a, 'b>>,
}

impl<'a, 'b> WithdrawContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<WithdrawContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let pool_market = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let pool_market_authority = AccountLoader::next_unchecked(account_info_iter)?; // Is PDA account of this program
        let pool = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let pool_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let withdrawal_requests = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let withdrawal_request = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let destination = AccountLoader::next_unchecked(account_info_iter)?; // Can be either spl or system (for native sol)
        let token_account = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let collateral_transit =
            AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let from = AccountLoader::next_unchecked(account_info_iter)?; // Request creator, can be any account
        let clock = AccountLoader::next_with_key(account_info_iter, &Clock::id())?;
        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;

        let native_sol_context = if AccountLoader::has_more(account_info_iter) {
            let token_mint =
                AccountLoader::next_with_key(account_info_iter, &spl_token::native_mint::id())?;
            let unwrap_sol = AccountLoader::next_uninitialized(account_info_iter)?;
            let signer = AccountLoader::next_signer(account_info_iter)?;
            let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;
            let _system_program =
                AccountLoader::next_with_key(account_info_iter, &system_program::id())?;

            Some(NativeSolContext {
                token_mint,
                unwrap_sol,
                signer,
                rent,
            })
        } else {
            None
        };

        Ok(WithdrawContext {
            pool_market,
            pool_market_authority,
            pool,
            pool_mint,
            withdrawal_requests,
            withdrawal_request,
            destination,
            token_account,
            collateral_transit,
            from,
            clock,
            native_sol_context,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        {
            // Check collateral token transit account
            let (collateral_transit_pubkey, _) =
                find_transit_program_address(program_id, self.pool_market.key, self.pool_mint.key);
            assert_account_key(self.collateral_transit, &collateral_transit_pubkey)?;
        }

        // We don't check the pool pda, because it's created from the program
        // and is linked to the pool market

        // Get pool state
        let pool = Pool::unpack(&self.pool.data.borrow())?;
        assert_account_key(self.pool_market, &pool.pool_market)?;
        assert_account_key(self.token_account, &pool.token_account)?;
        assert_account_key(self.pool_mint, &pool.pool_mint)?;

        // We don't check the withdrawal requests pda, because it's created from the program
        // and is linked to the pool
        let mut withdrawal_requests =
            WithdrawalRequests::unpack(&self.withdrawal_requests.data.borrow())?;
        assert_account_key(self.pool, &withdrawal_requests.pool)?;

        let withdrawal_request = WithdrawalRequest::unpack(&self.withdrawal_request.data.borrow())?;

        // Check withdraw request accounts
        assert_account_key(self.pool, &withdrawal_request.pool)?;
        assert_account_key(self.destination, &withdrawal_request.destination)?;
        assert_account_key(self.from, &withdrawal_request.from)?;

        // Check that enough time has passed to make a withdraw
        {
            let clock = Clock::from_account_info(self.clock)?;
            if withdrawal_request.ticket > clock.slot {
                return Err(EverlendError::WithdrawRequestsInvalidTicket.into());
            }
        }

        let (_, bump_seed) = find_program_address(program_id, self.pool_market.key);
        let signers_seeds = &[&self.pool_market.key.to_bytes()[..32], &[bump_seed]];

        // In the case of a SOL token, we do unwrap SPL token,
        // the destination can be any account

        if let Some(native_sol_context) = &self.native_sol_context {
            // Check transit: unwrapped sol
            let (unwrap_sol_pubkey, bump_seed) =
                find_transit_sol_unwrap_address(program_id, self.withdrawal_request.key);
            assert_account_key(native_sol_context.unwrap_sol, &unwrap_sol_pubkey)?;

            let unwrap_acc_signers_seeds = &[
                br"unwrap",
                &self.withdrawal_request.key.to_bytes()[..32],
                &[bump_seed],
            ];

            let rent = &Rent::from_account_info(native_sol_context.rent)?;

            cpi::system::create_account::<spl_token::state::Account>(
                &spl_token::id(),
                native_sol_context.signer.clone(),
                native_sol_context.unwrap_sol.clone(),
                &[unwrap_acc_signers_seeds],
                rent,
            )?;

            cpi::spl_token::initialize_account(
                native_sol_context.unwrap_sol.clone(),
                native_sol_context.token_mint.clone(),
                self.pool_market_authority.clone(),
                native_sol_context.rent.clone(),
            )?;

            // Transfer from token account to destination
            cpi::spl_token::transfer(
                self.token_account.clone(),
                native_sol_context.unwrap_sol.clone(),
                self.pool_market_authority.clone(),
                withdrawal_request.liquidity_amount,
                &[signers_seeds],
            )?;

            cpi::spl_token::close_account(
                native_sol_context.signer.clone(),
                native_sol_context.unwrap_sol.clone(),
                self.pool_market_authority.clone(),
                &[signers_seeds],
            )?;

            cpi::system::transfer(
                native_sol_context.signer.clone(),
                self.destination.clone(),
                withdrawal_request.liquidity_amount,
                &[],
            )?;
        } else {
            // Transfer from token account to destination
            cpi::spl_token::transfer(
                self.token_account.clone(),
                self.destination.clone(),
                self.pool_market_authority.clone(),
                withdrawal_request.liquidity_amount,
                &[signers_seeds],
            )?;
        };

        // Burn from transit collateral pool token
        cpi::spl_token::burn(
            self.pool_mint.clone(),
            self.collateral_transit.clone(),
            self.pool_market_authority.clone(),
            withdrawal_request.collateral_amount,
            &[signers_seeds],
        )?;

        withdrawal_requests.process(withdrawal_request.liquidity_amount)?;

        // Close withdraw account and return rent
        let from_starting_lamports = self.from.lamports();
        let withdraw_request_lamports = self.withdrawal_request.lamports();

        **self.withdrawal_request.lamports.borrow_mut() = 0;
        **self.from.lamports.borrow_mut() = from_starting_lamports
            .checked_add(withdraw_request_lamports)
            .ok_or(EverlendError::MathOverflow)?;

        WithdrawalRequests::pack(
            withdrawal_requests,
            *self.withdrawal_requests.data.borrow_mut(),
        )?;
        WithdrawalRequest::pack(
            Default::default(),
            *self.withdrawal_request.data.borrow_mut(),
        )?;

        Ok(())
    }
}
