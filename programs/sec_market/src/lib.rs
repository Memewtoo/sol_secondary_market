pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("EziFnVrPzQQQCoN6vYbJwwTyjps8QDrHh6G3fCQcyaDs");

#[program]
pub mod sec_market {
    use super::*;

    pub fn create_order(
        ctx: Context<CreateOrder>, 
        seed: u64, 
        price: u64, 
        amount: u64, 
        expiration: i64,
    ) -> Result<()> {

        create_order::handler(
            ctx, 
            seed, 
            price, 
            amount, 
            expiration)
    }

    pub fn cancel_order(ctx: Context<CancelOrder>, _seed: u64) -> Result<()> {
        cancel_order::handler(ctx)
    }

    pub fn settle_expired_order(ctx: Context<SettleExpiredOrder>, _seed: u64) -> Result<()> {
        settle_expired_order::handler(ctx)
    }

    pub fn modify_order(
        ctx: Context<ModifyOrder>,
        _seed: u64,
        new_amount: Option<u64>,
        new_price: Option<u64>,
        new_duration: Option<i64>
    ) -> Result<()> {

        modify_order::handler(ctx, new_amount, new_price, new_duration)
    }

    pub fn buy_tokens(ctx: Context<BuyTokens>, amount: u64) -> Result<()> {
        buy_tokens::handler(ctx, amount)
    }
}
