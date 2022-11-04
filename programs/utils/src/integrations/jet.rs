use solana_program::pubkey::Pubkey;

#[derive(Debug, Clone, Default)]
pub struct AccountPubkeys {
    pub margin_pool: Pubkey,
    pub vault: Pubkey,
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

    fn deposit_or_withdraw(_program_id: &Pubkey, pubkeys: &AccountPubkeys) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(pubkeys.margin_pool, false),
            AccountMeta::new(pubkeys.vault, false),
        ]
    }
}
