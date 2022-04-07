use anchor_lang::prelude::*;

#[derive(Accounts, Clone)]
pub struct InitPoolMarket<'info> {
    pub pool_market: AccountInfo<'info>,
    pub manager: AccountInfo<'info>,
    pub rent: AccountInfo<'info>,
}
