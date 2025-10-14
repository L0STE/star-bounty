use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Mint, Token, TokenAccount}, token_2022::Token2022
};
use cp_amm::{
    const_pda::pool_authority::ID as POOL_AUTHORITY_ID, constants::seeds::{CUSTOMIZABLE_POOL_PREFIX, POSITION_NFT_ACCOUNT_PREFIX, POSITION_PREFIX, TOKEN_VAULT_PREFIX }, instructions::initialize_pool::{max_key, min_key}, safe_math::SafeMath, state::{CollectFeeMode, Pool}
};
use ruint::aliases::U256;
use crate::state::InvestorFeePositionOwnerPda;
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        init,
        payer = payer,
        space = InvestorFeePositionOwnerPda::DISCRIMINATOR.len() + InvestorFeePositionOwnerPda::INIT_SPACE,
        seeds = [
            b"investor_fee_pos_owner", 
            mint_b.key().as_ref()
        ],
        bump,
    )]
    pub investor_fee_pos_owner: Account<'info, InvestorFeePositionOwnerPda>,

    // DAMM V2 Accounts
    #[account(mut)]
    pub payer: Signer<'info>,
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
        associated_token::authority = investor_fee_pos_owner,
    )]
    payer_token_a: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = investor_fee_pos_owner,
    )]
    payer_token_b: Box<Account<'info, TokenAccount>>,
    token_program: Program<'info, Token>,
    token_2022_program: Program<'info, Token2022>,
    system_program: Program<'info, System>,
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

impl<'info> Deposit<'info> {
    /// # Initialize Investor Fee Position Owner
    /// 
    /// This account will own the position in the pool and will be used to collect fees 
    /// and check for cranking every 24h.
    pub fn initialize_investor_fee_pos_owner(&mut self, bump: [u8; 1]) -> Result<()> {
        self.investor_fee_pos_owner.set_inner(
            InvestorFeePositionOwnerPda {
                associated_mint: self.mint_b.key(),
                last_claimed_at: 0,
                bump,
            }
        );

        Ok(())
    }

    /// # Deposit
    /// 
    /// This function will deposit the tokens into the pool and create the position.
    pub fn deposit(&mut self, position_nft_mint_bump: [u8; 1]) -> Result<()> {
        // Deserialize and check that the pool accept only fee in TokenB (our token)
        let pool = Pool::try_deserialize(&mut &self.pool.try_borrow_mut_data()?[..])?;
        if pool.collect_fee_mode != CollectFeeMode::OnlyB as u8 {
            return Err(ErrorCode::InvalidCollectFeeMode.into());
        }

        // Create the position
        let accounts = cp_amm::cpi::accounts::CreatePositionCtx {
            owner: self.investor_fee_pos_owner.to_account_info(),
            position_nft_mint: self.position_nft_mint.to_account_info(),
            position_nft_account: self.position_nft_account.to_account_info(),
            pool: self.pool.to_account_info(),
            position: self.position.to_account_info(),
            pool_authority: self.pool_authority.to_account_info(),
            payer: self.payer.to_account_info(),
            token_program: self.token_2022_program.to_account_info(),
            system_program: self.system_program.to_account_info(),
            event_authority: self.event_authority.to_account_info(),
            program: self.dammv2_program.to_account_info(),
        };

        let signer_seeds: [&[&[u8]];1] = [&[
            b"position_mint".as_ref(),
            self.investor_fee_pos_owner.to_account_info().key.as_ref(),
            &position_nft_mint_bump
        ]];

        let ctx = CpiContext::new_with_signer(
            self.dammv2_program.to_account_info(),
            accounts,
            &signer_seeds
        );

        cp_amm::cpi::create_position(ctx)?;

        // Add liquidity to the position
        let accounts = cp_amm::cpi::accounts::AddLiquidityCtx {
            owner: self.investor_fee_pos_owner.to_account_info(),
            pool: self.pool.to_account_info(),
            position: self.position.to_account_info(),
            token_a_account: self.payer_token_a.to_account_info(),
            token_b_account: self.payer_token_b.to_account_info(),
            token_a_vault: self.token_a_vault.to_account_info(),
            token_b_vault: self.token_b_vault.to_account_info(),
            token_a_mint: self.mint_a.to_account_info(),
            token_b_mint: self.mint_b.to_account_info(),
            position_nft_account: self.position_nft_account.to_account_info(),
            token_a_program: self.token_program.to_account_info(),
            token_b_program: self.token_program.to_account_info(),
            event_authority: self.event_authority.to_account_info(),
            program: self.dammv2_program.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.dammv2_program.to_account_info(),
            accounts,
            &signer_seeds
        );

        let liquidity_delta = Self::calculate_liquidity(
            self.payer_token_a.amount, 
            self.payer_token_b.amount, 
            pool.sqrt_price, 
            pool.sqrt_min_price, 
            pool.sqrt_max_price
        )?;

        cp_amm::cpi::add_liquidity(ctx, cp_amm::AddLiquidityParameters {
            liquidity_delta,
            token_a_amount_threshold: self.payer_token_a.amount,
            token_b_amount_threshold: self.payer_token_b.amount,
        })?;

        Ok(())
    }

    /// # Calculate Liquidity
    /// 
    /// An helper function to calculate the liquidity from the token amounts.
    pub fn calculate_liquidity(
        token_a_amount: u64,
        token_b_amount: u64,
        sqrt_price: u128,
        sqrt_min_price: u128,
        sqrt_max_price: u128,
    ) -> Result<u128> {
        // Calculate liquidity from token A
        // L = amount_a * sqrt_max_price * sqrt_price / (sqrt_max_price - sqrt_price)
        let l_from_a = U256::from(token_a_amount)
            .checked_mul(U256::from(sqrt_max_price))
            .and_then(|x| x.checked_mul(U256::from(sqrt_price)))
            .and_then(|x| x.checked_div(U256::from(sqrt_max_price - sqrt_price)))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Calculate liquidity from token B
        // L = amount_b * 2^128 / (sqrt_price - sqrt_min_price)
        let l_from_b = U256::from(token_b_amount)
            .safe_shl(128)?
            .checked_div(U256::from(sqrt_price - sqrt_min_price))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        // Take minimum to ensure we don't exceed either amount
        let liquidity = std::cmp::min(l_from_a, l_from_b);

        u128::try_from(liquidity).map_err(|_| ProgramError::ArithmeticOverflow.into())
    }
}