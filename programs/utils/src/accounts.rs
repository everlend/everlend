pub mod spl_token_lending {
    use crate::find_program_address;
    use solana_program::{instruction::AccountMeta, pubkey::Pubkey};

    pub fn deposit_or_redeem(
        reserve: &Pubkey,
        reserve_liquidity_supply: &Pubkey,
        lending_market: &Pubkey,
    ) -> Vec<AccountMeta> {
        let (lending_market_authority, _) =
            find_program_address(&spl_token_lending::id(), lending_market);

        vec![
            AccountMeta::new(*reserve, false),
            AccountMeta::new(*reserve_liquidity_supply, false),
            AccountMeta::new_readonly(*lending_market, false),
            AccountMeta::new_readonly(lending_market_authority, false),
        ]
    }
}
