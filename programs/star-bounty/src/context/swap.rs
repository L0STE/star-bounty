use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, Token, TokenAccount}
};
use cp_amm::{
    const_pda::pool_authority::ID as POOL_AUTHORITY_ID, constants::seeds::{CUSTOMIZABLE_POOL_PREFIX, TOKEN_VAULT_PREFIX }, instructions::initialize_pool::{max_key, min_key}, 
};

#[derive(Accounts)]
pub struct Swap<'info> {
    // DAMM V2 Accounts
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        address = POOL_AUTHORITY_ID
    )]
    /// CHECK: Account safely bound by address constraint
    pub pool_authority: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [
            CUSTOMIZABLE_POOL_PREFIX.as_ref(),
            &max_key(&mint_a.key(), &mint_b.key()),
            &min_key(&mint_a.key(), &mint_b.key()),
        ],
        seeds::program = cp_amm::ID,
        bump,
    )]
    /// CHECK: Account checked and initialized by DAMMV2
    pub pool: UncheckedAccount<'info>,
    pub mint_a: Box<Account<'info, Mint>>,
    pub mint_b: Box<Account<'info, Mint>>,
    #[account(
        mut,
        seeds = [
            TOKEN_VAULT_PREFIX.as_ref(),
            mint_a.key().as_ref(),
            pool.key().as_ref(),
        ],
        seeds::program = cp_amm::ID,
        bump,
    )]
    /// CHECK: Account checked and initialized by DAMMV2
    pub token_a_vault: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [
            TOKEN_VAULT_PREFIX.as_ref(),
            mint_b.key().as_ref(),
            pool.key().as_ref(),
        ],
        seeds::program = cp_amm::ID,
        bump,
    )]
    /// CHECK: Account checked and initialized by DAMMV2
    pub token_b_vault: UncheckedAccount<'info>,
    // Payer's token accounts
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = payer,
    )]
    pub payer_token_a: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = payer,
    )]
    pub payer_token_b: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    #[account(
        seeds = [b"__event_authority"], 
        seeds::program = cp_amm::ID,
        bump
    )]
    /// CHECK: Account checked and initialized by DAMMV2
    pub event_authority: AccountInfo<'info>,
    #[account(address = cp_amm::ID)]
    /// CHECK: Self-CPI will fail if the program is not the current program
    pub dammv2_program: AccountInfo<'info>,
}

impl<'info> Swap<'info> {
    pub fn swap_tokens(&mut self, amount: u64) -> Result<()> {
        // Sell Tokens
        let accounts = cp_amm::cpi::accounts::SwapCtx {
            pool_authority: self.pool_authority.to_account_info(),
            pool: self.pool.to_account_info(),
            input_token_account: self.payer_token_a.to_account_info(),
            output_token_account: self.payer_token_b.to_account_info(),
            token_a_vault: self.token_a_vault.to_account_info(),
            token_b_vault: self.token_b_vault.to_account_info(),
            token_a_mint: self.mint_a.to_account_info(),
            token_b_mint: self.mint_b.to_account_info(),
            payer: self.payer.to_account_info(),
            token_a_program: self.token_program.to_account_info(),
            token_b_program: self.token_program.to_account_info(),
            referral_token_account: None,
            event_authority: self.event_authority.to_account_info(),
            program: self.dammv2_program.to_account_info(),
            
        };

        let params = cp_amm::instructions::SwapParameters {
            amount_in: amount,
            minimum_amount_out: 0,
        };

        cp_amm::cpi::swap(
            CpiContext::new(
                self.dammv2_program.to_account_info(), 
                accounts
            ), params
        )?;

        Ok(())
    }
}