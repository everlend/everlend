use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke_signed,
    program_pack::Pack, pubkey::Pubkey, rent::Rent, system_instruction,
};

/// Create account
#[allow(clippy::too_many_arguments)]
pub fn create_account<'a, S: Pack>(
    program_id: &Pubkey,
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
    rent: &Rent,
) -> ProgramResult {
    let ix = system_instruction::create_account(
        from.key,
        to.key,
        rent.minimum_balance(S::LEN),
        S::LEN as u64,
        program_id,
    );

    invoke_signed(&ix, &[from, to], signers_seeds)
}

/// Transfer
#[allow(clippy::too_many_arguments)]
pub fn transfer<'a>(
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    lamports: u64,
    signers_seeds: &[&[&[u8]]],
) -> ProgramResult {
    let ix = system_instruction::transfer(from.key, to.key, lamports);

    invoke_signed(&ix, &[from, to], signers_seeds)
}
