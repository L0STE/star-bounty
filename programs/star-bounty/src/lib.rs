use anchor_lang::prelude::*;

declare_id!("3Wp5vW3yQeMTNKPxe4JCkEhFFrWdhw4tFt879tsMG2K9");

mod context;
use context::*;
mod state;
mod error;

pub const ADMIN: Pubkey = pubkey!("1oksyAnDKAFv4qgDDrwDh2XDHNM4vhXziXo5Qb5Rnmc");

#[program]
pub mod star_bounty {
    use super::*;

    
    pub fn initialize_pool(ctx: Context<InitializePool>) -> Result<()> {
        ctx.accounts.delegate_token_account([ctx.bumps.creator])?;
        ctx.accounts.create_pool([ctx.bumps.creator])?;
        ctx.accounts.revoke_token_account([ctx.bumps.creator])
    }

    pub fn create_stream(ctx: Context<CreateStream>) -> Result<()> {
        ctx.accounts.create_stream([ctx.bumps.metadata])
    }

    pub fn swap(ctx: Context<Swap>, amount: u64) -> Result<()> {
        ctx.accounts.swap_tokens(amount)
    }

    pub fn deposit(ctx: Context<Deposit>) -> Result<()> {
        ctx.accounts.initialize_investor_fee_pos_owner([ctx.bumps.investor_fee_pos_owner])?;
        ctx.accounts.deposit([ctx.bumps.position_nft_mint])
    }
    
    pub fn claim_fees<'info>(ctx: Context<'_, '_, '_, 'info, ClaimFees<'info>>) -> Result<()> {
        ctx.accounts.claim()?;
        ctx.accounts.distribute(ctx.remaining_accounts)
    }


}