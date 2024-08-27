use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer as SplTransfer};

declare_id!("4rz64UZebccmCfS5EZPSsPkraZNrYSdfqzbYgNtq8Roh");

#[program]
pub mod transfer_sol {
    use super::*;

    pub fn complete_transfer_payment(ctx: Context<CompleteTransferPayment>, payment: Payment) -> Result<()> {
        let from_ata = &ctx.accounts.from_ata;
        let to_ata = &ctx.accounts.to_ata;
        let token_program = &ctx.accounts.token_program;
        let payer = &ctx.accounts.payer;
        let treasury_ata = &ctx.accounts.treasury_ata;

        // Ensure the transaction has not expired
        if Clock::get()?.unix_timestamp > payment.expiry {
            return Err(ErrorCode::PaymentExpired.into());
        }

        // Transfer fee to the treasury account if there is any fee
        if payment.pay_in_amount > payment.pay_out_amount {
            let fee_amount = payment.pay_in_amount - payment.pay_out_amount;
            let cpi_accounts_fee = SplTransfer {
                from: from_ata.to_account_info(),
                to: treasury_ata.to_account_info(),
                authority: payer.to_account_info(),
            };
            let cpi_context_fee = CpiContext::new(token_program.to_account_info(), cpi_accounts_fee);
            token::transfer(cpi_context_fee, fee_amount)?;
        }

        // Transfer tokens from payer to merchant
        let cpi_accounts = SplTransfer {
            from: from_ata.to_account_info(),
            to: to_ata.to_account_info(),
            authority: payer.to_account_info(),
        };
        let cpi_context = CpiContext::new(token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_context, payment.pay_out_amount)?;

        // Emit an event after the successful payment
        emit!(PaymentCompleted {
            order_id: payment.order_id.clone(),
            pay_in_token: payment.pay_in_token,
            pay_out_token: payment.pay_out_token,
            pay_in_amount: payment.pay_in_amount,
            pay_out_amount: payment.pay_out_amount,
            fee_collected: payment.pay_in_amount - payment.pay_out_amount,
            merchant: payment.merchant,
        });

        Ok(())
    }

    pub fn redeem_fees(ctx: Context<RedeemFees>, amount: u64) -> Result<()> {
        let treasury_ata = &ctx.accounts.treasury_ata;
        let destination_ata = &ctx.accounts.destination_ata;
        let token_program = &ctx.accounts.token_program;
        let authority = &ctx.accounts.authority;

        // Transfer tokens from treasury to the specified destination
        let cpi_accounts = SplTransfer {
            from: treasury_ata.to_account_info(),
            to: destination_ata.to_account_info(),
            authority: authority.to_account_info(),
        };
        let cpi_context = CpiContext::new(token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_context, amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CompleteTransferPayment<'info> {
    pub payer: Signer<'info>,
    #[account(mut)]
    pub from_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub to_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub treasury_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct RedeemFees<'info> {
    pub authority: Signer<'info>,  // Authority to redeem fees
    #[account(mut)]
    pub treasury_ata: Account<'info, TokenAccount>,
    #[account(mut)]
    pub destination_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Payment {
    pub order_id: String,
    pub pay_in_token: Pubkey,
    pub pay_out_token: Pubkey,
    pub pay_in_amount: u64,
    pub pay_out_amount: u64,
    pub merchant: Pubkey,
    pub expiry: i64, // Unix timestamp for expiration
}

#[event]
pub struct PaymentCompleted {
    pub order_id: String,
    pub pay_in_token: Pubkey,
    pub pay_out_token: Pubkey,
    pub pay_in_amount: u64,
    pub pay_out_amount: u64,
    pub fee_collected: u64,
    pub merchant: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The payment has expired.")]
    PaymentExpired
}
