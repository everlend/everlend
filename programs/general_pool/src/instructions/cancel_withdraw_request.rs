use crate::{
    find_transit_program_address,
    state::{Pool, PoolMarket, WithdrawalRequest, WithdrawalRequests},
};
use everlend_utils::{
    assert_account_key, cpi, find_program_address, next_account, next_program_account,
    next_signer_account, next_unchecked_account, EverlendError,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey,
};

/// Instruction context
pub struct CancelWithdrawRequestContext<'a, 'b> {
    pool_market: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    withdrawal_requests: &'a AccountInfo<'b>,
    withdrawal_request: &'a AccountInfo<'b>,
    source: &'a AccountInfo<'b>,
    collateral_transit: &'a AccountInfo<'b>,
    pool_mint: &'a AccountInfo<'b>,
    pool_market_authority: &'a AccountInfo<'b>,
    from: &'a AccountInfo<'b>,
    manager: &'a AccountInfo<'b>,
}

impl<'a, 'b> CancelWithdrawRequestContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<CancelWithdrawRequestContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        let pool_market = next_account(account_info_iter, program_id)?;
        let pool = next_account(account_info_iter, program_id)?;
        let withdrawal_requests = next_account(account_info_iter, program_id)?;
        let withdrawal_request = next_account(account_info_iter, program_id)?;
        let source = next_account(account_info_iter, &spl_token::id())?;
        let collateral_transit = next_account(account_info_iter, &spl_token::id())?;
        let pool_mint = next_account(account_info_iter, &spl_token::id())?;
        let pool_market_authority = next_account(account_info_iter, program_id)?;
        let from = next_unchecked_account(account_info_iter)?; // we are checking later in code
        let manager = next_signer_account(account_info_iter)?;
        let _token_program = next_program_account(account_info_iter, &spl_token::id())?;

        Ok(CancelWithdrawRequestContext {
            pool_market,
            pool,
            withdrawal_requests,
            withdrawal_request,
            source,
            collateral_transit,
            pool_mint,
            pool_market_authority,
            from,
            manager,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        {
            let pool_market = PoolMarket::unpack(&self.pool_market.data.borrow())?;
            assert_account_key(self.manager, &pool_market.manager)?;

            // Get pool state
            let pool = Pool::unpack(&self.pool.data.borrow())?;
            assert_account_key(self.pool_market, &pool.pool_market)?;
            assert_account_key(self.pool_mint, &pool.pool_mint)?;

            // Check collateral token transit account
            let (collateral_transit_pubkey, _) =
                find_transit_program_address(program_id, self.pool_market.key, self.pool_mint.key);
            assert_account_key(self.collateral_transit, &collateral_transit_pubkey)?;
        }

        // We don't check the pool pda, because it's created from the program
        // and is linked to the pool market

        // We don't check the withdrawal requests pda, because it's created from the program
        // and is linked to the pool

        let mut withdrawal_requests =
            WithdrawalRequests::unpack(&self.withdrawal_requests.data.borrow())?;

        // Check withdrawal requests accounts
        assert_account_key(self.pool, &withdrawal_requests.pool)?;

        let withdrawal_request = WithdrawalRequest::unpack(&self.withdrawal_request.data.borrow())?;

        // Check withdrawal request accounts
        assert_account_key(self.pool, &withdrawal_request.pool)?;
        assert_account_key(self.source, &withdrawal_request.source)?;
        assert_account_key(self.from, &withdrawal_request.from)?;

        let (_, bump_seed) = find_program_address(program_id, self.pool_market.key);
        let signers_seeds = &[&self.pool_market.key.to_bytes()[..32], &[bump_seed]];

        cpi::spl_token::transfer(
            self.collateral_transit.clone(),
            self.source.clone(),
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
            withdrawal_request,
            *self.withdrawal_request.data.borrow_mut(),
        )?;

        Ok(())
    }
}
