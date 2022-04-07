use crate::*;
use anchor_lang::{prelude::*, solana_program};

pub fn init_pool_market<'a, 'b, 'c, 'info>(
    ctx: CpiContext<'a, 'b, 'c, 'info, InitPoolMarket<'info>>,
) -> Result<()> {
    let ix = everlend_general_pool::instruction::init_pool_market(
        ctx.program.key,
        ctx.accounts.pool_market.key,
        ctx.accounts.manager.key,
    );

    solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.pool_market,
            ctx.accounts.manager,
            ctx.accounts.rent,
        ],
        ctx.signer_seeds,
    )?;

    Ok(())
}
