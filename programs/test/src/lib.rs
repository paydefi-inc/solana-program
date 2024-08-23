use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("21JupgJvFJsU7uucmHdBqh8DsXSmg2Csc6ULq8MGeLav");

#[program]
mod ownership_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, owner: Pubkey) -> Result<()> {
        let ownership_account = &mut ctx.accounts.ownership_account;
        ownership_account.owner = owner;
        Ok(())
    }

    pub fn check_ownership(ctx: Context<CheckOwnership>, owner: Pubkey) -> Result<()> {
        let ownership_account = &ctx.accounts.ownership_account;
        if ownership_account.owner != owner {
            return Err(ErrorCode::InvalidOwner.into());
        }
        Ok(())
    }

    pub fn change_owner(ctx: Context<ChangeOwner>, new_owner: Pubkey) -> Result<()> {
        let ownership_account = &mut ctx.accounts.ownership_account;
        if  ctx.accounts.signer.key() != ownership_account.owner {
            return Err(ErrorCode::InvalidOwner.into());
        }
        ownership_account.owner = new_owner;
        Ok(())
    }

    pub fn complete_transfer_payment(
        ctx: Context<ReceiveFunds>,
        pay_in_amount: u64,
        pay_out_amount: u64,
        merchant: Pubkey,
    ) -> Result<()> {
        let fee_collected = pay_in_amount.checked_sub(pay_out_amount).ok_or(ErrorCode::Overflow)?;

        // Transfer the SPL token from the payer to the contract's account
        let cpi_accounts = Transfer {
            from: ctx.accounts.payer_token_account.to_account_info().clone(),
            to: ctx.accounts.contract_token_account.to_account_info().clone(),
            authority: ctx.accounts.payer.to_account_info().clone(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        token::transfer(CpiContext::new(cpi_program, cpi_accounts), pay_in_amount)?;

        // Derive the PDA (Program-Derived Address)
        let seeds = &[b"authority", &[ctx.accounts.contract_authority_bump]];
        let signer_seeds = &[&seeds[..]];

        // Transfer the pay_out_amount to the merchant's account using the PDA
        let cpi_accounts_merchant = Transfer {
            from: ctx.accounts.contract_token_account.to_account_info().clone(),
            to: ctx.accounts.merchant_token_account.to_account_info().clone(),
            authority: ctx.accounts.contract_authority.to_account_info().clone(),
        };
        token::transfer(
            CpiContext::new_with_signer(cpi_program, cpi_accounts_merchant, signer_seeds),
            pay_out_amount,
        )?;

        emit!(PaymentCompleted {
            order_id: ctx.accounts.order_id,
            pay_in_token: ctx.accounts.payer_token_account.mint,
            pay_in_amount,
            pay_out_amount,
            fee_collected,
            merchant,
        });

        Ok(())
    }

    pub fn withdraw_funds(ctx: Context<WithdrawFunds>, amount: u64) -> Result<()> {
        let ownership_account = &ctx.accounts.ownership_account;
        if ctx.accounts.signer.key() != ownership_account.owner {
            return Err(ErrorCode::InvalidOwner.into());
        }

        let transfer_instruction = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.program_account.to_account_info().key,
            &ctx.accounts.signer.key,
            amount,
        );
        anchor_lang::solana_program::program::invoke(
            &transfer_instruction,
            &[
                ctx.accounts.program_account.to_account_info(),
                ctx.accounts.signer.to_account_info(),
            ],
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = signer, space = 8 + 32)]
    pub ownership_account: Account<'info, OwnershipAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CheckOwnership<'info> {
    pub ownership_account: Account<'info, OwnershipAccount>,
}

#[derive(Accounts)]
pub struct ChangeOwner<'info> {
    #[account(mut)]
    pub ownership_account: Account<'info, OwnershipAccount>,
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct ReceiveFunds<'info> {
    #[account(mut)]
    pub payer_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub program_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub merchant_token_account: Account<'info, TokenAccount>,
    pub payer: Signer<'info>,
    #[account(seeds = [b"authority"], bump)]
    pub program_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
    pub order_id: u64,
    pub contract_authority_bump: u8, // PDA bump seed
}

#[derive(Accounts)]
pub struct WithdrawFunds<'info> {
    #[account(mut)]
    pub ownership_account: Account<'info, OwnershipAccount>,
    #[account(mut)]
    pub program_account: AccountInfo<'info>,
    pub signer: Signer<'info>,
}

#[account]
pub struct OwnershipAccount {
    pub owner: Pubkey,
}

#[event]
pub struct PaymentCompleted {
    pub order_id: u64,
    pub pay_in_token: Pubkey,
    pub pay_in_amount: u64,
    pub pay_out_amount: u64,
    pub fee_collected: u64,
    pub merchant: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The provided owner is not the current owner.")]
    InvalidOwner,
    #[msg("Overflow occurred during calculation.")]
    Overflow,
}
