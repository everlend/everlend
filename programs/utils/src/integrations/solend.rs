use solana_program::pubkey::Pubkey;

#[derive(Debug, Clone, Default)]
pub struct AccountPubkeys {
    pub reserve: Pubkey,
    pub reserve_liquidity_supply: Pubkey,
    pub reserve_liquidity_pyth_oracle: Pubkey,
    pub reserve_liquidity_switchboard_oracle: Pubkey,
    pub lending_market: Pubkey,
}

pub mod accounts {
    use crate::find_program_address;

    use super::AccountPubkeys;
    // use crate::find_program_address;
    use solana_program::{instruction::AccountMeta, pubkey::Pubkey};

    pub fn deposit(program_id: &Pubkey, pubkeys: &AccountPubkeys) -> Vec<AccountMeta> {
        deposit_or_withdraw(program_id, pubkeys)
    }

    pub fn withdraw(program_id: &Pubkey, pubkeys: &AccountPubkeys) -> Vec<AccountMeta> {
        deposit_or_withdraw(program_id, pubkeys)
    }

    fn deposit_or_withdraw(program_id: &Pubkey, pubkeys: &AccountPubkeys) -> Vec<AccountMeta> {
        let (lending_market_authority, _) =
            find_program_address(program_id, &pubkeys.lending_market);

        vec![
            AccountMeta::new(pubkeys.reserve, false),
            AccountMeta::new(pubkeys.reserve_liquidity_supply, false),
            AccountMeta::new_readonly(pubkeys.lending_market, false),
            AccountMeta::new_readonly(lending_market_authority, false),
            AccountMeta::new_readonly(pubkeys.reserve_liquidity_pyth_oracle, false),
            AccountMeta::new_readonly(pubkeys.reserve_liquidity_switchboard_oracle, false),
        ]
    }
}
