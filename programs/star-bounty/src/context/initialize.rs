use anchor_lang::prelude::*;
use anchor_spl::{
    token::{revoke, Revoke, approve, Approve, Token, Mint, TokenAccount}, 
    token_2022::Token2022, 
};

use cp_amm::{
    const_pda::pool_authority::ID as POOL_AUTHORITY_ID, constants::{seeds::{CUSTOMIZABLE_POOL_PREFIX, POSITION_NFT_ACCOUNT_PREFIX, POSITION_PREFIX, TOKEN_VAULT_PREFIX }, MAX_SQRT_PRICE, MIN_SQRT_PRICE}, cpi::accounts::InitializeCustomizablePoolCtx, instructions::initialize_pool::{max_key, min_key}, params::fee_parameters::{BaseFeeParameters, PoolFeeParameters}, safe_math::SafeMath, state::CollectFeeMode, InitializeCustomizablePoolParameters,
    utils_math::sqrt_u256,
};
use ruint::aliases::U256;
use crate::state::Creator;
use crate::error::ErrorCode;

const COMMITMENT_IN_BPS: u16 = 1_000; // 10%
const POOL_AMOUNT: u64 = 1_000_000_000_000; // 10% of a 10M token supply (with 6 decimals)

#[derive(Accounts)]
pub struct InitializePool<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = Creator::DISCRIMINATOR.len() + Creator::INIT_SPACE,
        seeds = [
            b"creator",
            mint_a.key().as_ref()
        ],
        bump
    )]
    pub creator: Account<'info, Creator>,
    #[account(
        mut,
        seeds = [
            b"position_mint", 
            creator.key().as_ref()
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
        associated_token::authority = creator,
    )]
    payer_token_a: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = creator,
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

impl<'info> InitializePool<'info> {
    pub fn populate_creator(&mut self, bump: [u8; 1]) -> Result<()> {
        self.creator.set_inner(Creator {
            bump,
            streams: 0,
        });

        Ok(())
    }
    
    pub fn delegate_token_account(&mut self, bump: [u8; 1]) -> Result<()> {
        let signer_seeds: [&[&[u8]];1] = [&[
            b"creator".as_ref(),
            self.mint_a.to_account_info().key.as_ref(),
            &bump
        ]];

        approve(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Approve {
                    to: self.payer_token_a.to_account_info(),
                    delegate: self.payer.to_account_info(),
                    authority: self.creator.to_account_info(),
                },
                &signer_seeds
            ),
            self.payer_token_a.amount
        )?;

        approve(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Approve {
                    to: self.payer_token_b.to_account_info(),
                    delegate: self.payer.to_account_info(),
                    authority: self.creator.to_account_info(),
                },
                &signer_seeds
            ),
            self.payer_token_b.amount
        )?;

        Ok(())
    }

    pub fn revoke_token_account(&mut self, bump: [u8; 1]) -> Result<()> {
        let signer_seeds: [&[&[u8]];1] = [&[
            b"creator".as_ref(),
            self.mint_a.to_account_info().key.as_ref(),
            &bump
        ]];

        revoke(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Revoke {
                    source: self.payer_token_a.to_account_info(),
                    authority: self.creator.to_account_info(),
                },
                &signer_seeds
            ),
        )?;

        revoke(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Revoke {
                    source: self.payer_token_b.to_account_info(),
                    authority: self.creator.to_account_info(),
                },
                &signer_seeds
            ),
        )?;

        Ok(())
    }

    pub fn create_pool(&mut self, position_nft_mint_bump: [u8;1]) -> Result<()> {
        let accounts = InitializeCustomizablePoolCtx {
            creator: self.creator.to_account_info(),
            position_nft_mint: self.position_nft_mint.to_account_info(),
            position_nft_account: self.position_nft_account.to_account_info(),
            payer: self.payer.to_account_info(),
            pool_authority: self.pool_authority.to_account_info(),
            pool: self.pool.to_account_info(),
            position: self.position.to_account_info(),
            token_a_mint: self.mint_a.to_account_info(),
            token_b_mint: self.mint_b.to_account_info(),
            token_a_vault: self.token_a_vault.to_account_info(),
            token_b_vault: self.token_b_vault.to_account_info(),
            payer_token_a: self.payer_token_a.to_account_info(),
            payer_token_b: self.payer_token_b.to_account_info(),
            token_a_program: self.token_program.to_account_info(),
            token_b_program: self.token_program.to_account_info(),
            token_2022_program: self.token_2022_program.to_account_info(),
            system_program: self.system_program.to_account_info(),
            event_authority: self.event_authority.to_account_info(),
            program: self.dammv2_program.to_account_info()
        };
    
        let signer_seeds: [&[&[u8]];1] = [&[
            b"position_mint".as_ref(),
            self.creator.to_account_info().key.as_ref(),
            &position_nft_mint_bump
        ]];
    
        let ctx = CpiContext::new_with_signer(
                self.dammv2_program.to_account_info(),
                accounts,
                &signer_seeds
        );
    
        let pool_fees = PoolFeeParameters {
            base_fee: BaseFeeParameters {
                cliff_fee_numerator: 2_500_000,
                first_factor: 0,
                second_factor: [0u8;8],
                third_factor: 0,
                base_fee_mode: 0,
            },
            padding: [0u8;3],
            dynamic_fee: None,
        };
    
        let amount_a = self.payer_token_a.amount
            .checked_mul(COMMITMENT_IN_BPS as u64)
            .and_then(|x| x.checked_div(10_000))
            .ok_or(ProgramError::ArithmeticOverflow)?;
    
        // Step 1: Estimate the initial sqrt price from token ratio
        let estimated_price_ratio = U256::from(amount_a)
            .safe_shl(128)?
            .checked_div(U256::from(POOL_AMOUNT))
            .ok_or(ProgramError::ArithmeticOverflow)?;
        
        let estimated_sqrt = u128::try_from(
            sqrt_u256(estimated_price_ratio)
                .ok_or(ProgramError::ArithmeticOverflow)?
        ).map_err(|_| ProgramError::ArithmeticOverflow)?;
    
        // Step 2: Create ±30% price range
        let range_sqrt_min = u128::try_from(
            U256::from(estimated_sqrt)
                .checked_mul(U256::from(8367_u32))  // sqrt(0.7) * 10000
                .and_then(|x| x.checked_div(U256::from(10000_u32)))
                .ok_or(ProgramError::ArithmeticOverflow)?
        ).map_err(|_| ProgramError::ArithmeticOverflow)?
            .max(MIN_SQRT_PRICE);
    
        let range_sqrt_max = u128::try_from(
            U256::from(estimated_sqrt)
                .checked_mul(U256::from(11402_u32))  // sqrt(1.3) * 10000
                .and_then(|x| x.checked_div(U256::from(10000_u32)))
                .ok_or(ProgramError::ArithmeticOverflow)?
        ).map_err(|_| ProgramError::ArithmeticOverflow)?
            .min(MAX_SQRT_PRICE);
    
        // Validate range
        require!(
            range_sqrt_min < range_sqrt_max,
            ErrorCode::InvalidAmount
        );
        require!(
            range_sqrt_min >= MIN_SQRT_PRICE && range_sqrt_max <= MAX_SQRT_PRICE,
            ErrorCode::InvalidAmount
        );
    
        // Step 3: Calculate the actual init price for exact ratio within the ±30% range
        let sqrt_price = Self::calculate_init_price(
            amount_a,
            POOL_AMOUNT,
            range_sqrt_min,
            range_sqrt_max
        )?;
        
        // Validate sqrt_price is within range
        require!(
            sqrt_price >= range_sqrt_min && sqrt_price <= range_sqrt_max,
            ErrorCode::InvalidAmount
        );
    
        // Step 4: Calculate correct liquidity for exact deposit amounts
        let liquidity = Self::calculate_liquidity(
            amount_a,
            POOL_AMOUNT,
            sqrt_price,
            range_sqrt_min,
            range_sqrt_max
        )?;
    
        // Verify liquidity is reasonable
        require!(liquidity > 0, ErrorCode::InvalidAmount);
    
        let params = InitializeCustomizablePoolParameters {
            pool_fees,
            sqrt_min_price: range_sqrt_min,
            sqrt_max_price: range_sqrt_max,
            has_alpha_vault: false,
            liquidity,
            sqrt_price, 
            activation_type: 0,
            collect_fee_mode: CollectFeeMode::OnlyB.into(),
            activation_point: None,
        };
    
        cp_amm::cpi::initialize_customizable_pool(
            ctx, 
            params
        )
    }

    pub fn calculate_init_price(
        token_a_amount: u64,
        token_b_amount: u64,
        min_sqrt_price: u128,
        max_sqrt_price: u128,
    ) -> Result<u128> {
        require!(
            token_a_amount != 0 && token_b_amount != 0, ErrorCode::InvalidAmount
        );
    
        let a = U256::from(token_a_amount);
        let b = U256::from(token_b_amount)
            .safe_shl(128)
            .map_err(|_| ProgramError::ArithmeticOverflow)?;
        let pa = U256::from(min_sqrt_price);
        let pb = U256::from(max_sqrt_price);
    
        let four = U256::from(4);
        let two = U256::from(2);
    
        let s = if b / a > pa * pb {
            let delta = b / a / pb - pa;
            let sqrt_value = sqrt_u256(delta * delta + four * b / a)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            (sqrt_value - delta) / two
        } else {
            let delta = pa - b / a / pb;
            let sqrt_value = sqrt_u256(delta * delta + four * b / a)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            (sqrt_value + delta) / two
        };
        Ok(u128::try_from(s).map_err(|_| ProgramError::ArithmeticOverflow)?)
    }

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