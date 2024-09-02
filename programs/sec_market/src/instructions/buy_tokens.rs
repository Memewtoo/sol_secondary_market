use anchor_lang::{prelude::*, system_program::{transfer as sol_transfer, Transfer as SOLTransfer}, solana_program::native_token::LAMPORTS_PER_SOL};
use anchor_spl::{associated_token::AssociatedToken, token::{transfer, Mint, Token, TokenAccount, Transfer}};

use crate::{state::Order, NATIVE_SOL_MINT};
use crate::error::Error;

#[derive(Accounts)]
pub struct BuyTokens<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

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
    pub price_token_mint: Account<'info, Mint>,

    #[account(mut)]
    pub escrow_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_token_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = vault_token_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_vault_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub buyer_price_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub creator_price_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<BuyTokens>, amount: u64) -> Result<()> {
    let order = &mut ctx.accounts.order;
    let price = order.price;

    require!(Clock::get()?.unix_timestamp <= order.expiration, Error::OrderExpired);
    require!(amount <= order.amount, Error::AmountExceedsAvailable);

    ctx.accounts.withdraw_from_vault(amount)?;
    ctx.accounts.transfer_price_to_creator(amount, price)?;
    ctx.accounts.update_or_close_order(amount)?;

    Ok(())
}

impl<'info> BuyTokens<'info>{
    pub fn withdraw_from_vault(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.escrow_token_account.to_account_info(),
            to: self.buyer_vault_token_account.to_account_info(),
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

        transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn transfer_price_to_creator(&mut self, amount: u64, price: u64) -> Result<()> {
        // Reference the price mint used in creating the order
        let price_mint = self.order.price_mint;

        let total_amount = price.checked_mul(amount).ok_or(Error::Overflow).unwrap();

        if price_mint == NATIVE_SOL_MINT {
            let cpi_program = self.system_program.to_account_info();

            let cpi_accounts = SOLTransfer {
                from: self.buyer.to_account_info(),
                to: self.creator.to_account_info(),
            };

            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

            let sol_total_amount = total_amount.checked_mul(LAMPORTS_PER_SOL).unwrap();

            sol_transfer(cpi_ctx, sol_total_amount)?;
        }

        else{
            let cpi_program = self.token_program.to_account_info();

            let cpi_accounts = Transfer {
                from: self.buyer_price_token_account.to_account_info(),
                to: self.creator_price_token_account.to_account_info(),
                authority: self.buyer.to_account_info()
            };

            let price_mint_total_amount = total_amount.checked_mul(10_u64.checked_pow(u32::from(self.price_token_mint.decimals)).unwrap()).unwrap();

            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

            transfer(cpi_ctx, price_mint_total_amount)?;
        }
        
        Ok(())
    }

    pub fn update_or_close_order (&mut self, amount: u64) -> Result<()> {
        // Closes the account if remaining_amount results to 0 after purchase, else just update it
        let order = &mut self.order;

        if order.remaining_amount.checked_sub(amount).unwrap() == 0 {
            let order_info = order.to_account_info();
            let dest_starting_lamports = self.creator.lamports();

            **self.creator.lamports.borrow_mut() = dest_starting_lamports.checked_add(order_info.lamports()).unwrap();
            **order_info.lamports.borrow_mut() = 0;

            order_info.assign(&self.system_program.key());
            order_info.realloc(0, false).map_err(|err| ProgramError::from(err))?;
        }
        else {
            order.remaining_amount = order.remaining_amount.checked_sub(amount).unwrap();
        }

        Ok(())
    }
}