use solana_program::pubkey::Pubkey;

#[derive(Debug, Clone, Default)]
pub struct AccountPubkeys {
    pub liquidity_pool: Pubkey,
    pub liquidity_owner: Pubkey,
    pub deposit_account: Pubkey,
    pub pool_admin: Pubkey,
}

pub mod accounts {
    use super::AccountPubkeys;
    use solana_program::{instruction::AccountMeta, pubkey::Pubkey};

    pub fn deposit(program_id: &Pubkey, pubkeys: &AccountPubkeys) -> Vec<AccountMeta> {
        deposit_or_withdraw(program_id, pubkeys)
    }

    pub fn withdraw(program_id: &Pubkey, pubkeys: &AccountPubkeys) -> Vec<AccountMeta> {
        deposit_or_withdraw(program_id, pubkeys)
    }

    fn deposit_or_withdraw(program_id: &Pubkey, pubkeys: &AccountPubkeys) -> Vec<AccountMeta> {
        let (unwrap_sol, _) = Pubkey::find_program_address(
            &[br"unwrap", &pubkeys.liquidity_pool.to_bytes()],
            program_id,
        );

        vec![
            AccountMeta::new(pubkeys.liquidity_pool, false),
            AccountMeta::new(pubkeys.liquidity_owner, false),
            AccountMeta::new(pubkeys.deposit_account, false),
            AccountMeta::new(pubkeys.pool_admin, false),
            AccountMeta::new(unwrap_sol, false),
            AccountMeta::new_readonly(spl_token::native_mint::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ]
    }
}
