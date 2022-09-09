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
use spl_token::state::Mint;

use crate::{
    find_pool_config_program_address, find_pool_program_address, find_transit_program_address,
    find_withdrawal_requests_program_address,
    state::{
        InitPoolParams, InitWithdrawalRequestsParams, Pool, PoolConfig, PoolMarket,
        WithdrawalRequests,
    },
    withdrawal_requests_seed,
};

/// Instruction context
pub struct CreatePoolContext<'a, 'b> {
    manager: &'a AccountInfo<'b>,
    pool_market: &'a AccountInfo<'b>,
    pool_config: &'a AccountInfo<'b>,
    token_mint: &'a AccountInfo<'b>,
    token_account: &'a AccountInfo<'b>,
    pool_mint: &'a AccountInfo<'b>,
    pool_market_authority: &'a AccountInfo<'b>,
    withdrawal_requests: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    transit: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
}

impl<'a, 'b> CreatePoolContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<CreatePoolContext<'a, 'b>, ProgramError> {
        let account_info_iter = &mut accounts.iter().enumerate();

        let pool_market = AccountLoader::next_with_owner(account_info_iter, program_id)?;
        let pool = AccountLoader::next_uninitialized(account_info_iter)?;
        let pool_config = AccountLoader::next_uninitialized(account_info_iter)?;
        let withdrawal_requests = AccountLoader::next_uninitialized(account_info_iter)?;
        let token_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let token_account = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let transit = AccountLoader::next_uninitialized(account_info_iter)?;
        let pool_mint = AccountLoader::next_with_owner(account_info_iter, &spl_token::id())?;
        let manager = AccountLoader::next_signer(account_info_iter)?;
        let pool_market_authority = AccountLoader::next_unchecked(account_info_iter)?; // Is PDA account of this program
        let rent = AccountLoader::next_with_key(account_info_iter, &Rent::id())?;
        let _system_program =
            AccountLoader::next_with_key(account_info_iter, &system_program::id())?;
        let _token_program = AccountLoader::next_with_key(account_info_iter, &spl_token::id())?;

        Ok(CreatePoolContext {
            manager,
            pool_market,
            token_mint,
            token_account,
            pool_mint,
            pool_market_authority,
            pool,
            transit,
            rent,
            pool_config,
            withdrawal_requests,
        })
    }

    /// Process instruction
    pub fn process(&self, program_id: &Pubkey) -> ProgramResult {
        // Check manager
        {
            let pool_market = PoolMarket::unpack(&self.pool_market.data.borrow())?;
            assert_account_key(self.manager, &pool_market.manager)?;
        }

        let token_mint = Mint::unpack(&self.token_mint.data.borrow())?;

        // Initialize token account for spl token
        cpi::spl_token::initialize_account(
            self.token_account.clone(),
            self.token_mint.clone(),
            self.pool_market_authority.clone(),
            self.rent.clone(),
        )?;

        // Initialize mint (token) for pool
        cpi::spl_token::initialize_mint(
            self.pool_mint.clone(),
            self.pool_market_authority.clone(),
            self.rent.clone(),
            token_mint.decimals,
        )?;

        let rent = &Rent::from_account_info(self.rent)?;

        self.create_pool(program_id, rent)?;
        self.create_transit(program_id, rent)?;
        self.create_withdrawal_requests(program_id, rent)?;
        self.create_pool_config(program_id, rent)?;

        Ok(())
    }

    fn create_pool(&self, program_id: &Pubkey, rent: &Rent) -> ProgramResult {
        // Create pool account
        let (pool_pubkey, pool_bump_seed) =
            find_pool_program_address(program_id, self.pool_market.key, self.token_mint.key);

        assert_account_key(self.pool, &pool_pubkey)?;

        let pool_signers_seeds = &[
            &self.pool_market.key.to_bytes()[..32],
            &self.token_mint.key.to_bytes()[..32],
            &[pool_bump_seed],
        ];

        cpi::system::create_account::<Pool>(
            program_id,
            self.manager.clone(),
            self.pool.clone(),
            &[pool_signers_seeds],
            rent,
        )?;

        let pool = Pool::init(InitPoolParams {
            pool_market: *self.pool_market.key,
            token_mint: *self.token_mint.key,
            token_account: *self.token_account.key,
            pool_mint: *self.pool_mint.key,
        });

        Pool::pack(pool, *self.pool.data.borrow_mut())
    }

    fn create_transit(&self, program_id: &Pubkey, rent: &Rent) -> ProgramResult {
        // Create transit account for SPL program
        let (transit_pubkey, transit_bump_seed) =
            find_transit_program_address(program_id, self.pool_market.key, self.pool_mint.key);
        assert_account_key(self.transit, &transit_pubkey)?;

        let transit_signers_seeds = &[
            br"transit",
            &self.pool_market.key.to_bytes()[..32],
            &self.pool_mint.key.to_bytes()[..32],
            &[transit_bump_seed],
        ];

        cpi::system::create_account::<spl_token::state::Account>(
            &spl_token::id(),
            self.manager.clone(),
            self.transit.clone(),
            &[transit_signers_seeds],
            rent,
        )?;

        // Initialize transit token account for spl token
        cpi::spl_token::initialize_account(
            self.transit.clone(),
            self.pool_mint.clone(),
            self.pool_market_authority.clone(),
            self.rent.clone(),
        )
    }

    fn create_withdrawal_requests(&self, program_id: &Pubkey, rent: &Rent) -> ProgramResult {
        // Check withdraw requests account
        let (withdrawal_requests_pubkey, bump_seed) = find_withdrawal_requests_program_address(
            program_id,
            self.pool_market.key,
            self.token_mint.key,
        );
        assert_account_key(self.withdrawal_requests, &withdrawal_requests_pubkey)?;

        let withdrawal_requests_seed = withdrawal_requests_seed();
        let signers_seeds = &[
            withdrawal_requests_seed.as_bytes(),
            &self.pool_market.key.to_bytes()[..32],
            &self.token_mint.key.to_bytes()[..32],
            &[bump_seed],
        ];

        cpi::system::create_account::<WithdrawalRequests>(
            program_id,
            self.manager.clone(),
            self.withdrawal_requests.clone(),
            &[signers_seeds],
            rent,
        )?;

        let withdrawal_requests = WithdrawalRequests::init(InitWithdrawalRequestsParams {
            pool: *self.pool.key,
            mint: *self.token_mint.key,
        });

        WithdrawalRequests::pack(
            withdrawal_requests,
            *self.withdrawal_requests.data.borrow_mut(),
        )
    }

    fn create_pool_config(&self, program_id: &Pubkey, rent: &Rent) -> ProgramResult {
        // Create Pool config
        let (pool_config_pubkey, bump_seed) =
            find_pool_config_program_address(program_id, self.pool.key);
        assert_account_key(self.pool_config, &pool_config_pubkey)?;

        let signers_seeds = &["config".as_bytes(), &self.pool.key.to_bytes(), &[bump_seed]];

        cpi::system::create_account::<PoolConfig>(
            program_id,
            self.manager.clone(),
            self.pool_config.clone(),
            &[signers_seeds],
            rent,
        )?;

        PoolConfig::pack(PoolConfig::default(), *self.pool_config.data.borrow_mut())
    }
}
