use std::str::FromStr;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token::Mint;
use anchor_spl::token::TokenAccount;
use anchor_spl::token::Token;
use anchor_spl::associated_token::AssociatedToken;
use streamflow_sdk::Create;

use crate::state::Creator;
use crate::ADMIN;

#[derive(Accounts)]
pub struct CreateStream<'info> {
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [
            b"creator",
            mint.key().as_ref(),
        ],
        bump = creator.bump[0]
    )]
    pub creator: Account<'info, Creator>,

    #[account(mut, address = ADMIN)]
    pub sender: Signer<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = sender,
    )]
    pub sender_tokens: Box<Account<'info, TokenAccount>>,
    /// CHECK: The admin pass this account so it's fine
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [
            b"metadata",
            creator.key().as_ref(),
            &[creator.streams],
        ],
        bump,
    )]
    /// CHECK: Checked by address constraint
    pub metadata: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [
            b"strm",
            metadata.key().as_ref(),
        ],
        seeds::program = streamflow_sdk::id(),
        bump,
    )]
    /// CHECK: Checked by address constraint
    pub escrow_tokens: UncheckedAccount<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = recipient,
    )]
    pub recipient_tokens: Box<Account<'info, TokenAccount>>,
    #[account(mut, address = Pubkey::from_str(streamflow_sdk::state::STRM_TREASURY).unwrap())]
    /// CHECK: Checked by address constraint
    pub streamflow_treasury: UncheckedAccount<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = streamflow_treasury,
    )]
    pub streamflow_treasury_tokens: Box<Account<'info, TokenAccount>>,
    #[account(mut, address = Pubkey::from_str(streamflow_sdk::state::WITHDRAWOR_ADDRESS).unwrap())]
    /// CHECK: Checked by address constraint
    pub withdrawor: UncheckedAccount<'info>,
    /// The Rent Sysvar account.
    pub rent: Sysvar<'info, Rent>,
    #[account(address = streamflow_sdk::id())]
    /// CHECK: Checked by address constraint
    pub timelock_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateStream<'info> {
    pub fn create_stream(&mut self, bump: [u8; 1]) -> Result<()> {
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"metadata",
            self.creator.to_account_info().key.as_ref(),
            &[self.creator.streams],
            &bump,
        ]];

        let ctx = CpiContext::new_with_signer(
            self.timelock_program.to_account_info(),
            streamflow_sdk::cpi::accounts::Create {
                sender: self.sender.to_account_info(),
                sender_tokens: self.sender_tokens.to_account_info(),
                recipient: self.recipient.to_account_info(),
                metadata: self.metadata.to_account_info(),
                escrow_tokens: self.escrow_tokens.to_account_info(),
                recipient_tokens: self.recipient_tokens.to_account_info(),
                streamflow_treasury: self.streamflow_treasury.to_account_info(),
                streamflow_treasury_tokens: self.streamflow_treasury_tokens.to_account_info(),
                withdrawor: self.withdrawor.to_account_info(),
                partner: self.streamflow_treasury.to_account_info(),
                partner_tokens: self.streamflow_treasury_tokens.to_account_info(),
                mint: self.mint.to_account_info(),
                fee_oracle: self.streamflow_treasury.to_account_info(),
                rent: self.rent.to_account_info(),
                timelock_program: self.timelock_program.to_account_info(),
                token_program: self.token_program.to_account_info(),
                associated_token_program: self.associated_token_program.to_account_info(),
                system_program: self.system_program.to_account_info(),
            },
            signer_seeds
        );

        streamflow_sdk::cpi::create(
            ctx, 
            Clock::get()?.unix_timestamp as u64, 
            1_000_000,
            60 * 60 * 24 * 30,
            1_000_000,
            0,
            0,
            true,
            false,
            true,
            true,
            false,
            true,
            [0; 64],
            60 * 60 * 24 * 30,
            None,
            None,
        )?;

        // Add a Stream
        self.creator.streams += 1;

        Ok(())
    }
}