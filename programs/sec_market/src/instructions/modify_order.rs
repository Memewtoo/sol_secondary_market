use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::state::Order;
use crate::error::Error;

#[derive(Accounts)]
pub struct ModifyOrder<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"order", 
            creator.key().as_ref(),
            order.seed.to_le_bytes().as_ref(),
            ],
        bump,
    )]
    pub order: Account<'info, Order>,

    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub creator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ModifyOrder>, new_amount: Option<u64>, new_price: Option<u64>, new_duration: Option<i64>) -> Result<()> {
    let order = &ctx.accounts.order;

    // Validates that the one cancelling the order is the one created it
    require!(order.creator == ctx.accounts.creator.key(), Error::Unauthorized);

    // Validates the that order isn't already partially filled
    require!(order.remaining_amount == order.amount, Error::OrderPartiallyFilled);

    ctx.accounts.modify_order(new_amount, new_price, new_duration)?;

    Ok(())
}

impl<'info> ModifyOrder<'info> {
    pub fn modify_order(&mut self, new_amount: Option<u64>, new_price: Option<u64>, new_duration: Option<i64>) -> Result<()> {

        if let Some(new_amount) = new_amount {
            if new_amount < self.order.amount {
                // Calculate the difference to transfer back
                let amount_to_return = self.order.amount - new_amount;

                self.withdraw_excess_tokens(amount_to_return)?;
                
            } else if new_amount > self.order.amount {

                // Calculate the difference to add to the escrow
                let amount_to_add = new_amount - self.order.amount;

                self.deposit_missing_tokens(amount_to_add)?;                
            }

            // Update the order with the new amount
            self.order.amount = new_amount;
            self.order.remaining_amount = new_amount;
        }

        if let Some(price) = new_price {
            self.order.price = price;
        }

        if let Some(duration) = new_duration {
            self.order.expiration = Clock::get()?.unix_timestamp + duration * 86400;
        }

        Ok(())
    }

    pub fn withdraw_excess_tokens(&mut self, amount: u64) -> Result<()> {

        // Prepare the CPI to transfer tokens back to the creator
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.escrow_token_account.to_account_info(),
            to: self.creator_token_account.to_account_info(),
            authority: self.order.to_account_info(), // authority of the escrow
        };

        let order_seed = self.order.seed.to_le_bytes();

        let seeds = &[
            b"order".as_ref(),
            self.creator.to_account_info().key.as_ref(),
            order_seed.as_ref(),
            &[self.order.order_bump],
        ];

        let signer_seeds = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        // Transfer the difference back to the creator
        transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn deposit_missing_tokens(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.creator_token_account.to_account_info(),
            to: self.escrow_token_account.to_account_info(),
            authority: self.creator.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(cpi_ctx, amount)?;
        
        Ok(())
    }
}