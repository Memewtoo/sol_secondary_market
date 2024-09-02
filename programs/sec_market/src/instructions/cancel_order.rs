use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::state::Order;
use crate::error::Error;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct CancelOrder<'info> {
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

pub fn handler(ctx: Context<CancelOrder>) -> Result<()> {
    let order = &ctx.accounts.order;

    // Validates that the one cancelling the order is the one created it
    require!(order.creator == ctx.accounts.creator.key(), Error::Unauthorized);

    // Validates the that order isn't already partially filled
    require!(order.remaining_amount == order.amount, Error::OrderPartiallyFilled);

    ctx.accounts.withdraw_tokens()?;

    Ok(())
}

impl<'info> CancelOrder<'info> {
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