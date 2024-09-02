use anchor_lang::prelude::*;

#[error_code]
pub enum Error {
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,

    #[msg("The order has expired.")]
    OrderExpired,

    #[msg("The order is not expired yet.")]
    OrderNotExpired,

    #[msg("Amount exceeds available tokens.")]
    AmountExceedsAvailable,

    #[msg("Order has been partially filled and cannot be modified.")]
    OrderPartiallyFilled,

    #[msg("Overflow error.")]
    Overflow,
}
