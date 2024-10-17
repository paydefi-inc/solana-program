use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer as SplTransfer};
use raydium_cp_swap::{cpi, program::RaydiumCpSwap, states::{AmmConfig, ObservationState, PoolState}};

declare_id!("2ohN4V8zMjE63ggB777fSjWcDTWkqMsUvycjxzJKTEqp");

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
            from: from_ata.key(),
            treasury: treasury_ata.key(),
            merchant: payment.merchant,
            payer: payer.key()
        });

        Ok(())
    }

    pub fn complete_transfer_donation(ctx: Context<CompleteTransferPayment>, payment: Payment) -> Result<()> {
        let from_ata = &ctx.accounts.from_ata;
        let to_ata = &ctx.accounts.to_ata;
        let token_program = &ctx.accounts.token_program;
        let payer = &ctx.accounts.payer;
        let treasury_ata = &ctx.accounts.treasury_ata;

        if Clock::get()?.unix_timestamp > payment.expiry {
            return Err(ErrorCode::PaymentExpired.into());
        }

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

        let cpi_accounts = SplTransfer {
            from: from_ata.to_account_info(),
            to: to_ata.to_account_info(),
            authority: payer.to_account_info(),
        };
        let cpi_context = CpiContext::new(token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_context, payment.pay_out_amount)?;

        emit!(DonationCompleted {
            order_id: payment.order_id.clone(),
            pay_in_token: payment.pay_in_token,
            pay_out_token: payment.pay_out_token,
            pay_in_amount: payment.pay_in_amount,
            pay_out_amount: payment.pay_out_amount,
            fee_collected: payment.pay_in_amount - payment.pay_out_amount,
            from: from_ata.key(),
            treasury: treasury_ata.key(),
            merchant: payment.merchant,
            payer: payer.key()
        });

        Ok(())
    }
}

pub fn complete_swap_payment(ctx: Context<CompleteSwapPayment>, payment: Payment) -> Result<()> {
    let payer = &ctx.accounts.payer;
    let from_ata = &ctx.accounts.from_ata;
    // let payer_output_ata = &ctx.accounts.payer_output_ata;
    let merchant_ata = &ctx.accounts.merchant_ata;
    let treasury_ata = &ctx.accounts.treasury_ata;
    let token_program = &ctx.accounts.token_program;

    // Ensure the transaction has not expired
    if Clock::get()?.unix_timestamp > payment.expiry {
        return Err(ErrorCode::PaymentExpired.into());
    }

    // Step 1: Perform the swap
    let swap_amount = payment.pay_in_amount;
    let cpi_accounts = cpi::accounts::Swap {
        payer: payer.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
        amm_config: ctx.accounts.amm_config.to_account_info(),
        pool_state: ctx.accounts.pool_state.to_account_info(),
        input_token_account: from_ata.to_account_info(),
        output_token_account: ctx.accounts.payer_output_ata.to_account_info(),
        input_vault: ctx.accounts.input_vault.to_account_info(),
        output_vault: ctx.accounts.output_vault.to_account_info(),
        input_token_program: token_program.to_account_info(),
        output_token_program: token_program.to_account_info(),
        input_token_mint: ctx.accounts.input_mint.to_account_info(),
        output_token_mint: ctx.accounts.output_mint.to_account_info(),
        observation_state: ctx.accounts.observation_state.to_account_info(),
    };
    let cpi_context = CpiContext::new(ctx.accounts.cp_swap_program.to_account_info(), cpi_accounts);

    // Swap tokens from from_ata to payer_output_ata
    cpi::swap_base_input(cpi_context, swap_amount, 0)?;

    // Step 2: Transfer pay_out_amount to the merchant
    let cpi_accounts_transfer = SplTransfer {
        from: ctx.accounts.payer_output_ata.to_account_info(),
        to: merchant_ata.to_account_info(),
        authority: payer.to_account_info(),
    };
    let cpi_context_transfer = CpiContext::new(token_program.to_account_info(), cpi_accounts_transfer);
    token::transfer(cpi_context_transfer, payment.pay_out_amount)?;

    // Reload the payer's output token account to get the updated balance
    let payer_output_ata = &mut ctx.accounts.payer_output_ata;
    payer_output_ata.reload()?;

    // Step 3: Transfer fee to the treasury (if any)
    let remaining_balance = payer_output_ata.amount;
    let fee_amount = remaining_balance;

    if fee_amount > 0 {
        let cpi_accounts_fee = SplTransfer {
            from: payer_output_ata.to_account_info(),
            to: treasury_ata.to_account_info(),
            authority: payer.to_account_info(),
        };
        let cpi_context_fee = CpiContext::new(token_program.to_account_info(), cpi_accounts_fee);
        token::transfer(cpi_context_fee, fee_amount)?;
    }

    // Emit an event after the successful payment
    emit!(SwapTransferCompleted {
        order_id: payment.order_id.clone(),
        pay_in_token: payment.pay_in_token,
        pay_out_token: payment.pay_out_token,
        pay_in_amount: payment.pay_in_amount,
        pay_out_amount: payment.pay_out_amount,
        fee_collected: fee_amount,
        from: from_ata.key(),
        treasury: treasury_ata.key(),
        merchant: payment.merchant,
        payer: payer.key()
    });

    Ok(())
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
#[instruction(payment: Payment)]
pub struct CompleteSwapPayment<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    // Payer's input token account (from which tokens will be swapped)
    #[account(
        mut,
        constraint = from_ata.owner == payer.key(),
    )]
    pub from_ata: Account<'info, TokenAccount>,

    // Payer's output token account (where swapped tokens are received)
    #[account(
        mut,
        constraint = payer_output_ata.owner == payer.key(),
    )]
    pub payer_output_ata: Account<'info, TokenAccount>,

    // Merchant's token account (receiving payment)
    #[account(mut)]
    pub merchant_ata: Account<'info, TokenAccount>,

    // Treasury account for fee collection
    #[account(mut)]
    pub treasury_ata: Account<'info, TokenAccount>,

    // Swap-related accounts
    /// CHECK: Pool authority
    pub authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub pool_state: AccountLoader<'info, PoolState>,

    #[account(address = pool_state.load()?.amm_config)]
    pub amm_config: Box<Account<'info, AmmConfig>>,

    #[account(mut)]
    pub input_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub output_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = pool_state.load()?.observation_key)]
    pub observation_state: AccountLoader<'info, ObservationState>,

    pub cp_swap_program: Program<'info, RaydiumCpSwap>,

    pub token_program: Program<'info, Token>,

    // Mints
    pub input_mint: Account<'info, Mint>,

    pub output_mint: Account<'info, Mint>,
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
    pub from: Pubkey,
    pub treasury: Pubkey,
    pub merchant: Pubkey,
    pub payer: Pubkey
}

#[event]
pub struct DonationCompleted {
    pub order_id: String,
    pub pay_in_token: Pubkey,
    pub pay_out_token: Pubkey,
    pub pay_in_amount: u64,
    pub pay_out_amount: u64,
    pub fee_collected: u64,
    pub from: Pubkey,
    pub treasury: Pubkey,
    pub merchant: Pubkey,
    pub payer: Pubkey
}

#[event]
pub struct SwapTransferCompleted {
    pub order_id: String,
    pub pay_in_token: Pubkey,
    pub pay_out_token: Pubkey,
    pub pay_in_amount: u64,
    pub pay_out_amount: u64,
    pub fee_collected: u64,
    pub from: Pubkey,
    pub treasury: Pubkey,
    pub merchant: Pubkey,
    pub payer: Pubkey
}

#[error_code]
pub enum ErrorCode {
    #[msg("The payment has expired.")]
    PaymentExpired,

    #[msg("Swap failed.")]
    SwapFailed,

    #[msg("Insufficient funds in payer's output token account.")]
    InsufficientFunds,
}
