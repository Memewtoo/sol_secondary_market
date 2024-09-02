use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::state::Order;
use crate::error::Error;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct SettleExpiredOrder<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"order", 
            creator.key().as_ref(),
            seed.to_le_bytes().as_ref(),
            ],
        bump,
        close = creator
    )]
    pub order: Account<'info, Order>,

    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub creator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<SettleExpiredOrder>) -> Result<()> {
    let order = &ctx.accounts.order;

    // Validates that the order is expired
    require!(Clock::get()?.unix_timestamp > order.expiration, Error::OrderNotExpired);

    ctx.accounts.withdraw_tokens()?;

    Ok(())
}

impl<'info> SettleExpiredOrder<'info> {
    pub fn withdraw_tokens(&mut self) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.escrow_token_account.to_account_info(),
            to: self.creator_token_account.to_account_info(),
            authority: self.order.to_account_info(),
        };

        let order_seed = self.order.seed.to_le_bytes();

        let seeds = &[
            &b"order"[..],
            self.creator.to_account_info().key.as_ref(),
            order_seed.as_ref(),
            &[self.order.order_bump],
        ];

        let signer_seeds = &[&seeds[..]];


        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        transfer(cpi_ctx, self.order.amount)?;

        Ok(())
    }
}