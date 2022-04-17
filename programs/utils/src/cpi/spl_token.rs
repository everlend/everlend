use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
};

/// Initialize SPL mint instruction.
pub fn initialize_mint<'a>(
    mint: AccountInfo<'a>,
    mint_authority: AccountInfo<'a>,
    rent: AccountInfo<'a>,
    decimals: u8,
) -> ProgramResult {
    let ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        mint.key,
        mint_authority.key,
        None,
        decimals,
    )?;

    invoke(&ix, &[mint, rent])
}

/// Initialize SPL accont instruction.
pub fn initialize_account<'a>(
    account: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    rent: AccountInfo<'a>,
) -> ProgramResult {
    let ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        account.key,
        mint.key,
        authority.key,
    )?;

    invoke(&ix, &[account, mint, authority, rent])
}

/// SPL transfer instruction.
pub fn transfer<'a>(
    source: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = spl_token::instruction::transfer(
        &spl_token::id(),
        source.key,
        destination.key,
        authority.key,
        &[],
        amount,
    )?;

    invoke_signed(&ix, &[source, destination, authority], signers_seeds)
}

/// SPL mint instruction.
pub fn mint_to<'a>(
    mint: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint.key,
        destination.key,
        authority.key,
        &[],
        amount,
    )?;

    invoke_signed(&ix, &[mint, destination, authority], signers_seeds)
}

/// SPL burn instruction.
pub fn burn<'a>(
    mint: AccountInfo<'a>,
    account: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    amount: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = spl_token::instruction::burn(
        &spl_token::id(),
        account.key,
        mint.key,
        authority.key,
        &[],
        amount,
    )?;

    invoke_signed(&ix, &[mint, account, authority], signers_seeds)
}

/// SPL close account instruction.
pub fn close_account<'a>(
    destination: AccountInfo<'a>,
    account: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = spl_token::instruction::close_account(
        &spl_token::id(),
        account.key,
        destination.key,
        authority.key,
        &[],
    )?;

    invoke_signed(&ix, &[account, destination, authority], signers_seeds)
}
