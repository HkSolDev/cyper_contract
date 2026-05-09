use anchor_lang::prelude::*;
use crate::state::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use anchor_spl::associated_token::AssociatedToken;


#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    // Stable Coin Ata for Treasury
    #[account(
        init,
        payer = authority,
        associated_token::mint = mint,
        associated_token::authority = authority,
        associated_token::token_program = token_program
    )]
    pub treasury: InterfaceAccount<'info, TokenAccount>,

    #[account(zero)]
    pub market: Account<'info, CyperMarket>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn handler(ctx: Context<Initialize>, fee:u16) -> Result<()> {
    
    msg!("Greetings from: {:?}", ctx.program_id);
    let market = &mut ctx.accounts.market;
    market.authority = ctx.accounts.authority.key();
    market.treasury = ctx.accounts.treasury.key();
    market.default_protocol_fee_bps = fee;
    market.market_count = 0;
    Ok(())
}
