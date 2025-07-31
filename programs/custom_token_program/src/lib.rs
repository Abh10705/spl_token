
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, FreezeAccount, ThawAccount, MintTo, Approve};

// This is the program's on-chain ID. Anchor automatically populates this.
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod custom_token_program {
    use super::*;

    // Instruction 1: Create a new token mint.
    // This sets our program's PDA as the freeze authority.
    pub fn create_token_mint(
        ctx: Context<CreateTokenMint>,
        decimals: u8,
        mint_authority: Pubkey,
    ) -> Result<()> {
        // We don't need to do anything here.
        // Anchor's framework, combined with the account constraints below,
        // handles the creation and initialization of the mint account.
        // The `#[account(...)` macros are doing the heavy lifting.
        // Specifically, the `mint` account is being created and initialized
        // by the token program, with the `freeze_authority` set to our PDA.
        Ok(())
    }

    // Instruction 2: Delegate spending authority to another account.
    // This is a direct wrapper around the SPL Token program's `approve` instruction.
    pub fn delegate_tokens(ctx: Context<DelegateTokens>, amount: u64) -> Result<()> {
        let cpi_accounts = Approve {
            to: ctx.accounts.token_account.to_account_info(),
            delegate: ctx.accounts.delegate.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        
        token::approve(cpi_ctx, amount)?;
        
        Ok(())
    }

    // Instruction 3: Freeze a user's token account.
    pub fn freeze_token_account(ctx: Context<FreezeOrThawAccount>) -> Result<()> {
        // Security Check: Ensure the signer is the original mint authority.
        // This prevents unauthorized accounts from freezing tokens.
        require_keys_eq!(ctx.accounts.admin.key(), ctx.accounts.mint.mint_authority.unwrap(), CustomError::Unauthorized);

        let cpi_accounts = FreezeAccount {
            account: ctx.accounts.token_account_to_process.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            authority: ctx.accounts.program_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        
        // We need to provide the PDA seeds for the program to "sign" the transaction.
        let seeds = &["authority".as_bytes(), &[ctx.bumps.program_authority]];
        let signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::freeze_account(cpi_ctx)?;
        
        Ok(())
    }

    // Instruction 4: Thaw (unfreeze) a user's token account.
    pub fn thaw_token_account(ctx: Context<FreezeOrThawAccount>) -> Result<()> {
        // Security Check: Ensure the signer is the original mint authority.
        require_keys_eq!(ctx.accounts.admin.key(), ctx.accounts.mint.mint_authority.unwrap(), CustomError::Unauthorized);

        let cpi_accounts = ThawAccount {
            account: ctx.accounts.token_account_to_process.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            authority: ctx.accounts.program_authority.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        
        // We use the same PDA seeds to sign for the thaw operation.
        let seeds = &["authority".as_bytes(), &[ctx.bumps.program_authority]];
        let signer = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::thaw_account(cpi_ctx)?;
        
        Ok(())
    }
}

// ====== Account Structs ======

#[derive(Accounts)]
#[instruction(decimals: u8, mint_authority: Pubkey)]
pub struct CreateTokenMint<'info> {
    #[account(
        init,
        payer = payer,
        mint::decimals = decimals,
        mint::authority = mint_authority,
        // CRITICAL: This sets our program's PDA as the freeze authority.
        mint::freeze_authority = program_authority.key()
    )]
    pub mint: Account<'info, Mint>,

    /// CHECK: This is our program's authority, a PDA. It doesn't need to be checked because we are defining it here.
    #[account(
        seeds = [b"authority"],
        bump
    )]
    pub program_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct DelegateTokens<'info> {
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    
    /// CHECK: The account being delegated to. It can be any account.
    pub delegate: UncheckedAccount<'info>,
    
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct FreezeOrThawAccount<'info> {
    // The authority allowed to freeze/thaw (e.g., the original creator of the token).
    pub admin: Signer<'info>,

    #[account(mut)]
    pub token_account_to_process: Account<'info, TokenAccount>,
    
    // We need the mint to verify that the admin is the mint_authority.
    #[account(
        constraint = mint.key() == token_account_to_process.mint
    )]
    pub mint: Account<'info, Mint>,

    /// CHECK: This is the same PDA from our CreateTokenMint instruction.
    #[account(
        seeds = [b"authority"],
        bump
    )]
    pub program_authority: UncheckedAccount<'info>,
    
    pub token_program: Program<'info, Token>,
}

// ====== Custom Error ======

#[error_code]
pub enum CustomError {
    #[msg("Unauthorized: The signer is not the mint authority.")]
    Unauthorized,
}
