use anchor_lang::prelude::*;

#[account]
pub struct Order {
    pub seed: u64,
    pub creator: Pubkey,
    pub amount: u64,
    pub remaining_amount: u64,
    pub price: u64,
    pub price_mint: Pubkey, //Token Mint used for pricing (e.g., USDC, USDT, SOL)
    pub expiration: i64,
    pub order_bump: u8,
}

impl Space for Order {
    const INIT_SPACE: usize = 
        8 +  // ACCOUNT DISCRIMINATOR
        8 +  // SEED
        32 + // CREATOR PUBKEY
        8 +  // AMOUNT
        8 +  // REMAINING_AMOUNT
        8 +  // PRICE
        32 + // PRICE_MINT PUBKEY
        8 +  // EXPIRATION
        1;   // ORDER_BUMP
}