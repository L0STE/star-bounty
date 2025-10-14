use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Creator {
    //.. other fields
    pub streams: u8,
    pub bump: [u8; 1],
}

#[account]
#[derive(InitSpace)]
pub struct InvestorFeePositionOwnerPda {
    pub associated_mint: Pubkey,
    pub last_claimed_at: i64,
    pub bump: [u8; 1],
}