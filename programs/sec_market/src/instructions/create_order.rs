use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{transfer, Mint, Token, TokenAccount, Transfer}};

use crate::state::Order;
use crate::error::Error;

#[derive(Accounts)]
#[instruction(seed:u64)]
pub struct CreateOrder<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        space = Order::INIT_SPACE,  //account discriminator, creator pkey, amount, price, expiration
        seeds = [
            b"order", 
            creator.key().as_ref(),
            seed.to_le_bytes().as_ref(),
            ],
        bump,
    )]
    pub order: Account<'info, Order>,

    #[account(mut)]
    pub vault_token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = creator,
        associated_token::mint = vault_token_mint,
        associated_token::authority = order,
    )]
    pub escrow_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub price_token_mint: Account<'info, Mint>,

    #[account(mut)]
    pub creator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CreateOrder>, seed: u64, price: u64, amount: u64, expiration: i64) -> Result<()> {

    ctx.accounts.create_order(seed, price, amount, expiration, &ctx.bumps)?;
    ctx.accounts.deposit_tokens()?;

    Ok(())
}

impl<'info> CreateOrder<'info> {
    pub fn create_order(&mut self, seed: u64, price: u64, amount: u64, expiration: i64, bumps: &CreateOrderBumps) -> Result<()> {
        let expiration_seconds = expiration
            .checked_mul(86400) // duration in days to seconds
            .ok_or(Error::Overflow)?;

        let expiration_timestamp = Clock::get()?.unix_timestamp
            .checked_add(expiration_seconds)
            .ok_or(Error::Overflow)?;

        self.order.set_inner(Order {
            seed,
            creator: self.creator.key(),
            price,
            amount,
            remaining_amount: amount,
            price_mint: self.price_token_mint.key(),
            expiration: expiration_timestamp,
            order_bump: bumps.order,
        });

        Ok(())
    }

    pub fn deposit_tokens(&mut self) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.creator_token_account.to_account_info(),
            to: self.escrow_token_account.to_account_info(),
            authority: self.creator.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(cpi_ctx, self.order.amount)?;

        Ok(())
    }
}
