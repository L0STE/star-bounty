use std::str::FromStr;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::token::Mint;
use anchor_spl::token::TokenAccount;
use anchor_spl::token::Token;
use anchor_spl::associated_token::AssociatedToken;

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

        let accounts = vec![
            AccountMeta::new(self.sender.key(), true),
            AccountMeta::new(self.sender_tokens.key(), false),
            AccountMeta::new(self.recipient.key(), false),
            AccountMeta::new(self.metadata.key(), true),
            AccountMeta::new(self.escrow_tokens.key(), false),
            AccountMeta::new(self.recipient_tokens.key(), false),
            AccountMeta::new(self.streamflow_treasury.key(), false),
            AccountMeta::new(self.streamflow_treasury_tokens.key(), false),
            AccountMeta::new(self.withdrawor.key(), false),
            AccountMeta::new(self.streamflow_treasury.key(), false),                // partner
            AccountMeta::new(self.streamflow_treasury_tokens.key(), false),         // partner_tokens
            AccountMeta::new_readonly(self.mint.key(), false),
            AccountMeta::new_readonly(self.streamflow_treasury.key(), false),       // fee_oracle
            AccountMeta::new_readonly(self.rent.key(), false),
            AccountMeta::new_readonly(self.timelock_program.key(), false),
            AccountMeta::new_readonly(self.token_program.key(), false),
            AccountMeta::new_readonly(self.associated_token_program.key(), false),
            AccountMeta::new_readonly(self.system_program.key(), false),
        ];

        let instruction_data = streamflow_sdk::instruction::Create {
            start_time: Clock::get()?.unix_timestamp as u64,
            net_amount_deposited: 1_000_000,
            period: 60 * 60 * 24 * 30,
            amount_per_period: 1_000_000,
            cliff: 0,
            cliff_amount: 0,
            cancelable_by_sender: true,
            cancelable_by_recipient: false,
            automatic_withdrawal: true,
            transferable_by_sender: true,
            transferable_by_recipient: false,
            can_topup: true,
            stream_name: [0; 64],
            withdraw_frequency: 60 * 60 * 24 * 30,
            pausable: None,
            can_update_rate: None,
        }.try_to_vec()?;

        let account_infos = &[
            self.sender.to_account_info(),
            self.sender_tokens.to_account_info(),
            self.recipient.to_account_info(),
            self.metadata.to_account_info(),
            self.escrow_tokens.to_account_info(),
            self.recipient_tokens.to_account_info(),
            self.streamflow_treasury.to_account_info(),
            self.streamflow_treasury_tokens.to_account_info(),
            self.withdrawor.to_account_info(),
            self.mint.to_account_info(),
            self.rent.to_account_info(),
            self.timelock_program.to_account_info(),
            self.token_program.to_account_info(),
            self.associated_token_program.to_account_info(),
            self.system_program.to_account_info(),
        ];
        
        let instruction = anchor_lang::solana_program::instruction::Instruction {
            program_id: streamflow_sdk::id(),
            accounts,
            data: instruction_data,
        };

        invoke_signed(&instruction, account_infos, signer_seeds)?;

        // Add a Stream
        self.creator.streams += 1;

        Ok(())
    }
}