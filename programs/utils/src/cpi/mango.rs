use solana_program::account_info::AccountInfo;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

const ACCOUNT_NUM: u64 = 1;

pub fn find_account_program_address(
    program_id: &Pubkey,
    mango_group: &Pubkey,
    owner: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            mango_group.as_ref(),
            owner.as_ref(),
            &ACCOUNT_NUM.to_le_bytes(),
        ],
        program_id,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn deposit<'a>(
    program_id: &Pubkey,
    mango_group: AccountInfo<'a>,
    mango_account: AccountInfo<'a>,
    owner: AccountInfo<'a>,
    mango_cache: AccountInfo<'a>,
    root_bank: AccountInfo<'a>,
    node_bank: AccountInfo<'a>,
    vault: AccountInfo<'a>,
    owner_token_account: AccountInfo<'a>,
    quantity: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = mango::instruction::deposit(
        program_id,
        mango_group.key,
        mango_account.key,
        owner.key,
        mango_cache.key,
        root_bank.key,
        node_bank.key,
        vault.key,
        owner_token_account.key,
        quantity,
    )?;

    invoke_signed(
        &ix,
        &[
            mango_group,
            mango_account,
            owner,
            mango_cache,
            root_bank,
            node_bank,
            vault,
            owner_token_account,
        ],
        signers_seeds,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn withdraw<'a>(
    program_id: &Pubkey,
    mango_group: AccountInfo<'a>,
    mango_account: AccountInfo<'a>,
    owner: AccountInfo<'a>,
    mango_cache: AccountInfo<'a>,
    root_bank: AccountInfo<'a>,
    node_bank: AccountInfo<'a>,
    vault: AccountInfo<'a>,
    owner_token_account: AccountInfo<'a>,
    quantity: u64,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = mango::instruction::withdraw(
        program_id,
        mango_group.key,
        mango_account.key,
        owner.key,
        mango_cache.key,
        root_bank.key,
        node_bank.key,
        vault.key,
        owner_token_account.key,
        owner.key,
        &[],
        quantity,
        false,
    )?;

    invoke_signed(
        &ix,
        &[
            mango_group,
            mango_account,
            owner,
            mango_cache,
            root_bank,
            node_bank,
            vault,
            owner_token_account,
        ],
        signers_seeds,
    )
}

pub fn create_mango_account<'a>(
    program_id: &Pubkey,
    mango_group: AccountInfo<'a>,
    mango_account: AccountInfo<'a>,
    owner: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
) -> Result<(), ProgramError> {
    let ix = mango::instruction::create_mango_account(
        program_id,
        mango_group.key,
        mango_account.key,
        owner.key,
        &system_program::id(),
        owner.key,
        ACCOUNT_NUM,
    )?;

    invoke_signed(&ix, &[mango_group, mango_account, owner], signers_seeds)
}
