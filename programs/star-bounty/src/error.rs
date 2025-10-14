use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid collect fee mode")]
    InvalidCollectFeeMode,
    #[msg("No vested amount available")]
    NoVestedAmount,
}