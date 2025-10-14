use anchor_lang::prelude::*;
use anchor_spl::{
    token::{transfer, Mint, Token, TokenAccount, Transfer}
};
use cp_amm::{const_pda::pool_authority::ID as POOL_AUTHORITY_ID, constants::seeds::{CUSTOMIZABLE_POOL_PREFIX, POSITION_NFT_ACCOUNT_PREFIX, POSITION_PREFIX, TOKEN_VAULT_PREFIX}, max_key, min_key};

use crate::state::{InvestorFeePositionOwnerPda, Creator};

const DUST_THRESHOLD: u64 = 1_000_000; // 1 Token
const MAX_INVESTOR_SHARE_BPS: u16 = 1_000; // 10%
const DAILY_CAP: u64 = 1_000_000_000; // 1000 Tokens

#[derive(Accounts)]
pub struct ClaimFees<'info> {
    pub mint_a: Box<Account<'info, Mint>>,
    pub mint_b: Box<Account<'info, Mint>>,
    #[account(
        seeds = [
            b"creator",
            mint_b.key().as_ref(),
        ],
        bump = creator.bump[0],
    )]
    pub creator: Account<'info, Creator>,
    pub creator_token_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        seeds = [
            b"investor_fee_pos_owner", 
            mint_b.key().as_ref()
        ],
        bump = investor_fee_pos_owner.bump[0],
    )]
    pub investor_fee_pos_owner: Account<'info, InvestorFeePositionOwnerPda>,
    #[account(
        mut,
        seeds = [
            b"position_mint", 
            investor_fee_pos_owner.key().as_ref()
        ],
        bump,
    )]
    /// CHECK: Account checked and initialized by DAMMV2. Must be either a Signer or CPI Signer
    pub position_nft_mint: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [
            POSITION_NFT_ACCOUNT_PREFIX.as_ref(), 
            position_nft_mint.key().as_ref()
        ],
        seeds::program = cp_amm::ID,
        bump
    )]
    /// CHECK: Account checked and initialized by DAMMV2
    pub position_nft_account: UncheckedAccount<'info>,
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
    #[account(
        mut,
        seeds = [
            POSITION_PREFIX.as_ref(),
            position_nft_mint.key().as_ref()
        ],
        seeds::program = cp_amm::ID,
        bump,
    )]
    /// CHECK: Account checked and initialized by DAMMV2
    pub position: UncheckedAccount<'info>,
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
        associated_token::authority = investor_fee_pos_owner,
    )]
    pub token_a_account: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = investor_fee_pos_owner,
    )]
    pub token_b_account: Box<Account<'info, TokenAccount>>,
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

impl<'info> ClaimFees<'info> {
    pub fn claim(&mut self) -> Result<()> {
        let signer_seeds: [&[&[u8]];1] = [&[
            b"investor_fee_pos_owner".as_ref(),
            self.mint_b.to_account_info().key.as_ref(),
            &self.investor_fee_pos_owner.bump
        ]];

        let accounts = cp_amm::cpi::accounts::ClaimPositionFeeCtx {
            pool_authority: self.pool_authority.to_account_info(),
            pool: self.pool.to_account_info(),
            position: self.position.to_account_info(),
            token_a_account: self.token_a_account.to_account_info(),
            token_b_account: self.token_b_account.to_account_info(),
            token_a_vault: self.token_a_vault.to_account_info(),
            token_b_vault: self.token_b_vault.to_account_info(),
            token_a_mint: self.mint_a.to_account_info(),
            token_b_mint: self.mint_b.to_account_info(),
            position_nft_account: self.position_nft_account.to_account_info(),
            owner: self.investor_fee_pos_owner.to_account_info(),
            token_a_program: self.token_program.to_account_info(),
            token_b_program: self.token_program.to_account_info(),
            event_authority: self.event_authority.to_account_info(),
            program: self.dammv2_program.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(self.dammv2_program.to_account_info(), accounts, &signer_seeds);

        cp_amm::cpi::claim_position_fee(ctx)?;

        Ok(())
    }

    pub fn distribute(&mut self, remaining_accounts: &[AccountInfo<'info>]) -> Result<()> {
        let streams = self.creator.streams as usize;

        require_eq!(remaining_accounts.len(), streams * 2);

        let now = Clock::get()?.unix_timestamp;

        // Check if 24h have passed since last claim and update last claimed at
        require_gte!(now, self.investor_fee_pos_owner.last_claimed_at + 86400);        
        self.investor_fee_pos_owner.last_claimed_at = now;

        self.token_b_account.reload()?;

        // Skip distribution if there's only Dust in the account
        if self.token_b_account.amount < DUST_THRESHOLD {
            return Ok(());
        }

        let mut initial_locked: u64 = 0;                            // Y0
        let mut total_locked: u64 = 0;
        let mut locked_amounts: Vec<u64> = Vec::new();

        // Validate streams and collect LOCKED amounts and total locked
        for i in 0..streams {
            let metadata_account = &remaining_accounts[i * 2];
            let token_account = &remaining_accounts[i * 2 + 1];

            let (expected_stream, _) = Pubkey::find_program_address(&[
                b"metadata",
                self.creator.to_account_info().key.as_ref(),
                &[i as u8],
            ], &crate::ID);
            require_eq!(expected_stream, metadata_account.key());

            let stream_metadata = streamflow_sdk::state::Contract::deserialize(&mut metadata_account.try_borrow_data()?.as_ref())?;
            require_eq!(stream_metadata.recipient_tokens, token_account.key());

            // Grab the total locked amount at the start of the stream
            initial_locked = initial_locked.checked_add(stream_metadata.ix.net_amount_deposited).ok_or(ProgramError::ArithmeticOverflow)?;

            // Calculate the locked amount at the current time
            let locked = stream_metadata.ix.net_amount_deposited
                .checked_sub(stream_metadata.vested_available(now as u64))
                .and_then(|x| x.checked_sub(stream_metadata.amount_withdrawn))
                .ok_or(ProgramError::ArithmeticOverflow)?;
            locked_amounts.push(locked);

            // Update the total locked amount
            total_locked = total_locked.checked_add(locked).ok_or(ProgramError::ArithmeticOverflow)?;
        }

        // Calulate the eligibile investor share
        let f_locked_bps = (total_locked as u128)
            .checked_mul(10_000)
            .and_then(|x| x.checked_div(initial_locked as u128))
            .ok_or(ProgramError::ArithmeticOverflow)?;
        
        // Cacluate the investor share
        let eligible_investor_share_bps = std::cmp::min(MAX_INVESTOR_SHARE_BPS as u128, f_locked_bps);
        let investor_fee_quote = (self.token_b_account.amount as u128)
            .checked_mul(eligible_investor_share_bps)
            .and_then(|x| x.checked_div(10_000))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Calculate the daily cap
        let distributable = std::cmp::min(investor_fee_quote, DAILY_CAP as u128);

        // Distribute fees pro-rata based on vested amounts
        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"investor_fee_pos_owner".as_ref(),
            self.mint_b.to_account_info().key.as_ref(),
            &self.investor_fee_pos_owner.bump
        ]];

        // Distribute fees pro-rata based on vested amounts
        for i in 0..streams {
            let locked = locked_amounts[i];
            if locked == 0 {
                continue;
            }
        
            // weight_i = locked_i / total_locked
            let share = (distributable as u128)
                .checked_mul(locked as u128)
                .and_then(|x| x.checked_div(total_locked as u128))
                .ok_or(ProgramError::ArithmeticOverflow)?;
            
            if share == 0 {
                continue;
            }
        
            transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.token_b_account.to_account_info(),
                        to: remaining_accounts[i * 2 + 1].to_account_info(),
                        authority: self.investor_fee_pos_owner.to_account_info(),
                    },
                    &signer_seeds,
                ),
                u64::try_from(share)?
            )?;
        }

        // Send remainder to creator
        self.token_b_account.reload()?;

        if self.token_b_account.amount > 0 {
            transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    Transfer {
                        from: self.token_b_account.to_account_info(),
                        to: self.creator_token_account.to_account_info(),
                        authority: self.investor_fee_pos_owner.to_account_info(),
                    },
                    &signer_seeds,
                ),
                self.token_b_account.amount
            )?;
        }

        Ok(())
    }
}