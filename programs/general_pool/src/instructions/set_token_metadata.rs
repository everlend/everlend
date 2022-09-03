use crate::state::{Pool, PoolMarket};
use everlend_utils::{
    assert_account_key, cpi::metaplex, find_program_address, next_account, next_optional_account,
    next_program_account, next_signer_account,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::Pack, pubkey::Pubkey, rent::Rent, system_program, sysvar::SysvarId,
};

/// Instruction context
pub struct SetTokenMetadataContext<'a, 'b> {
    manager: &'a AccountInfo<'b>,
    metadata: &'a AccountInfo<'b>,
    metaplex_program: &'a AccountInfo<'b>,
    pool: &'a AccountInfo<'b>,
    pool_market: &'a AccountInfo<'b>,
    pool_market_authority: &'a AccountInfo<'b>,
    pool_mint: &'a AccountInfo<'b>,
    rent: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
}

impl<'a, 'b> SetTokenMetadataContext<'a, 'b> {
    /// New instruction context
    pub fn new(
        program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> Result<SetTokenMetadataContext<'a, 'b>, ProgramError> {
        let metaplex_program_id = metaplex::program_id();

        let account_info_iter = &mut accounts.iter();
        let pool_market = next_account(account_info_iter, program_id)?;
        let pool = next_account(account_info_iter, program_id)?;
        let pool_mint = next_account(account_info_iter, &spl_token::id())?;
        let pool_market_authority = next_account(account_info_iter, program_id)?;
        let metadata = next_optional_account(account_info_iter, &metaplex_program_id)?;
        let manager = next_signer_account(account_info_iter)?;
        let metaplex_program = next_program_account(account_info_iter, &metaplex_program_id)?;
        let system_program = next_program_account(account_info_iter, &system_program::id())?;
        let rent = next_program_account(account_info_iter, &Rent::id())?;

        Ok(SetTokenMetadataContext {
            manager,
            metadata,
            metaplex_program,
            pool,
            pool_market,
            pool_market_authority,
            pool_mint,
            rent,
            system_program,
        })
    }

    /// Process instruction
    pub fn process(
        &self,
        program_id: &Pubkey,
        name: String,
        symbol: String,
        uri: String,
    ) -> ProgramResult {
        {
            // Get pool market state
            let pool_market = PoolMarket::unpack(&self.pool_market.data.borrow())?;
            assert_account_key(self.manager, &pool_market.manager)?;

            // Get pool state
            let pool = Pool::unpack(&self.pool.data.borrow())?;
            assert_account_key(self.pool_market, &pool.pool_market)?;
            assert_account_key(self.pool_mint, &pool.pool_mint)?;
        }

        // Get authority
        let (pool_market_authority, bump_seed) =
            find_program_address(program_id, self.pool_market.key);
        assert_account_key(self.pool_market_authority, &pool_market_authority)?;

        let signers_seeds = &[&self.pool_market.key.to_bytes()[..32], &[bump_seed]];

        if self.metadata.owner.eq(&Pubkey::default()) {
            metaplex::create_metadata(
                self.metaplex_program.clone(),
                self.metadata.clone(),
                self.pool_mint.clone(),
                self.pool_market_authority.clone(),
                self.manager.clone(),
                self.system_program.clone(),
                self.rent.clone(),
                name,
                symbol,
                uri,
                &[signers_seeds],
            )?;
        } else {
            metaplex::update_metadata(
                self.metaplex_program.clone(),
                self.metadata.clone(),
                self.pool_mint.clone(),
                self.pool_market_authority.clone(),
                name,
                symbol,
                uri,
                &[signers_seeds],
            )?;
        }

        Ok(())
    }
}
