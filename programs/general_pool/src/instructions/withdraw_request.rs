use crate::{
    find_pool_config_program_address, find_pool_program_address, find_transit_program_address,
    find_withdrawal_request_program_address,
    state::{
        InitWithdrawalRequestParams, Pool, PoolConfig, WithdrawalRequest, WithdrawalRequests,
        WITHDRAW_DELAY,
    },
    utils::total_pool_amount,
};
use everlend_utils::{
    assert_account_key, assert_owned_by, cpi, cpi::rewards::withdraw_mining, next_account,
    next_optional_account, next_program_account, next_signer_account, next_uninitialized_account,
    EverlendError,
};
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
use spl_token::state::{Account, Mint};

/// Instruction context
pub struct WithdrawRequestContext<'a, 'b> {
    pool_config: &'a AccountInfo<'b>,
    pool_market: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    pool_mint: &'a AccountInfo<'b>,
    withdrawal_requests: &'a AccountInfo<'b>,
    withdrawal_request: &'a AccountInfo<'b>,
    source: &'a AccountInfo<'b>,
    destination: &'a AccountInfo<'b>,
    token_account: &'a AccountInfo<'b>,
    collateral_transit: &'a AccountInfo<'b>,
    user_transfer_authority: &'a AccountInfo<'b>,
    mining_reward_pool: &'a AccountInfo<'b>,
    mining_reward_acc: &'a AccountInfo<'b>,
    everlend_config: &'a AccountInfo<'b>,
    everlend_rewards_program: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
    clock: &'a AccountInfo<'b>,
}

impl<'a, 'b> WithdrawRequestContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<WithdrawRequestContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter();

        let pool_config = next_optional_account(account_info_iter, program_id)?;
        let pool_market = next_account(account_info_iter, program_id)?;
        let pool = next_account(account_info_iter, program_id)?;
        let pool_mint = next_account(account_info_iter, &spl_token::id())?;
        let withdrawal_requests = next_account(account_info_iter, program_id)?;
        let withdrawal_request = next_uninitialized_account(account_info_iter)?;
        let source = next_account(account_info_iter, &spl_token::id())?;
        let destination = next_account(account_info_iter, &spl_token::id())?;
        let token_account = next_account(account_info_iter, &spl_token::id())?;
        let collateral_transit = next_account(account_info_iter, &spl_token::id())?;
        let user_transfer_authority = next_signer_account(account_info_iter)?;
        let mining_reward_pool = next_account(account_info_iter, &eld_rewards::id())?;
        let mining_reward_acc = next_account(account_info_iter, &eld_rewards::id())?;
        let everlend_config = next_account(account_info_iter, &eld_config::id())?;
        let everlend_rewards_program = next_program_account(account_info_iter, &eld_rewards::id())?;
        let rent = next_program_account(account_info_iter, &Rent::id())?;
        let clock = next_program_account(account_info_iter, &Clock::id())?;
        let _system_program = next_program_account(account_info_iter, &system_program::id())?;
        let _token_program = next_program_account(account_info_iter, &spl_token::id())?;

        Ok(WithdrawRequestContext {
            pool_config,
            pool_market,
            pool,
            pool_mint,
            withdrawal_requests,
            withdrawal_request,
            source,
            destination,
            token_account,
            collateral_transit,
            user_transfer_authority,
            mining_reward_pool,
            mining_reward_acc,
            everlend_config,
            everlend_rewards_program,
            rent,
            clock,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey, collateral_amount: u64) -> ProgramResult {
        let pool = Pool::unpack(&self.pool.data.borrow())?;

        // Check pool accounts
        assert_account_key(self.pool_market, &pool.pool_market)?;
        assert_account_key(self.token_account, &pool.token_account)?;
        assert_account_key(self.pool_mint, &pool.pool_mint)?;

        // In all cases except SOL token, we must check destination account
        if pool.token_mint != spl_token::native_mint::id() {
            let destination_account = Account::unpack(&self.destination.data.borrow())?;
            if pool.token_mint != destination_account.mint {
                return Err(ProgramError::InvalidArgument);
            }
        }

        // Check transit: collateral
        let (collateral_transit_pubkey, _) =
            find_transit_program_address(program_id, self.pool_market.key, self.pool_mint.key);
        assert_account_key(self.collateral_transit, &collateral_transit_pubkey)?;

        let mut withdrawal_requests =
            WithdrawalRequests::unpack(&self.withdrawal_requests.data.borrow())?;

        // Check withdrawal requests accounts
        assert_account_key(self.pool, &withdrawal_requests.pool)?;

        // Check withdrawal request
        let (withdrawal_request_pubkey, bump_seed) = find_withdrawal_request_program_address(
            program_id,
            self.withdrawal_requests.key,
            self.user_transfer_authority.key,
        );
        assert_account_key(self.withdrawal_request, &withdrawal_request_pubkey)?;

        let total_incoming =
            total_pool_amount(self.token_account.clone(), pool.total_amount_borrowed)?;
        let total_minted = Mint::unpack_unchecked(&self.pool_mint.data.borrow())?.supply;

        let liquidity_amount = (collateral_amount as u128)
            .checked_mul(total_incoming as u128)
            .ok_or(EverlendError::MathOverflow)?
            .checked_div(total_minted as u128)
            .ok_or(EverlendError::MathOverflow)? as u64;

        let (pool_config_pubkey, _) = find_pool_config_program_address(program_id, self.pool.key);
        assert_account_key(self.pool_config, &pool_config_pubkey)?;

        if !self.pool_config.owner.eq(&Pubkey::default()) {
            assert_owned_by(self.pool_config, program_id)?;

            let pool_config = PoolConfig::unpack(&self.pool_config.data.borrow())?;
            if liquidity_amount < pool_config.withdraw_minimum {
                return Err(EverlendError::WithdrawAmountTooSmall.into());
            }
        }

        // Transfer
        cpi::spl_token::transfer(
            self.source.clone(),
            self.collateral_transit.clone(),
            self.user_transfer_authority.clone(),
            collateral_amount,
            &[],
        )?;

        let signers_seeds = &[
            br"withdrawal",
            &self.withdrawal_requests.key.to_bytes()[..32],
            &self.user_transfer_authority.key.to_bytes()[..32],
            &[bump_seed],
        ];

        let rent = &Rent::from_account_info(self.rent)?;
        let clock = Clock::from_account_info(self.clock)?;

        cpi::system::create_account::<WithdrawalRequest>(
            program_id,
            self.user_transfer_authority.clone(),
            self.withdrawal_request.clone(),
            &[signers_seeds],
            rent,
        )?;

        let mut withdrawal_request =
            WithdrawalRequest::unpack_unchecked(&self.withdrawal_request.data.borrow())?;

        withdrawal_request.init(InitWithdrawalRequestParams {
            pool: *self.pool.key,
            from: *self.user_transfer_authority.key,
            source: *self.source.key,
            destination: *self.destination.key,
            liquidity_amount,
            collateral_amount,
            ticket: clock.slot + WITHDRAW_DELAY,
        });

        withdrawal_requests.add(liquidity_amount)?;

        WithdrawalRequests::pack(
            withdrawal_requests,
            *self.withdrawal_requests.data.borrow_mut(),
        )?;
        WithdrawalRequest::pack(
            withdrawal_request,
            *self.withdrawal_request.data.borrow_mut(),
        )?;

        // Mining reward
        let (pool_pubkey, pool_bump_seed) =
            find_pool_program_address(program_id, &pool.pool_market, &pool.token_mint);
        assert_account_key(self.pool, &pool_pubkey)?;

        let pool_seeds: &[&[u8]] = &[
            &pool.pool_market.to_bytes()[..32],
            &pool.token_mint.to_bytes()[..32],
            &[pool_bump_seed],
        ];

        withdraw_mining(
            self.everlend_rewards_program.key,
            self.everlend_config.clone(),
            self.mining_reward_pool.clone(),
            self.mining_reward_acc.clone(),
            self.user_transfer_authority.clone(),
            self.pool.to_owned(),
            collateral_amount,
            &[pool_seeds],
        )?;

        Ok(())
    }
}
