use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{Token, TokenAccount, Transfer as SplTransfer}};

pub mod amm_instruction;

// devnet address
declare_id!("Hbi7DiU48QgKSyJevjFraKak9fLXJea1uNrDLiwxJNkz");

#[program]
pub mod paydefi {
    use amm_instruction::swap_base_in;
    use anchor_spl::token;
    use solana_program::program::invoke;
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
            treasury: treasury_ata.key(),
            merchant: payment.merchant,
            payer: payer.key()
        });

        Ok(())
    }

    pub fn complete_swap_payment(ctx: Context<CompletePayment>, payment: Payment) -> Result<()> {
        let accts: &mut CompletePayment<'_> = ctx.accounts;

        let payer = &accts.payer;

        let treasury_ata = &accts.treasury_ata;

        // Ensure the transaction has not expired
        if Clock::get()?.unix_timestamp > payment.expiry {
            return Err(ErrorCode::PaymentExpired.into());
        }
        //  Get initial to ata amount
        let initial_ata_balance: u64 = accts.to_ata.amount;

        //  Swap pay_in_token to pay_out_token via Raydium CPI
        let swap_ix = swap_base_in(
            &accts.raydium_amm_program.key(),
            &accts.amm_id.key(),
            &accts.amm_authority.key(),
            &accts.amm_open_orders.key(),
            &accts.pool_coin_token_account.key(),
            &accts.pool_pc_token_account.key(),
            &accts.serum_program.key(),
            &accts.serum_market.key(),
            &accts.serum_bids.key(),
            &accts.serum_asks.key(),
            &accts.serum_event_queue.key(),
            &accts.serum_coin_vault.key(),
            &accts.serum_pc_vault.key(),
            &accts.serum_vault_signer.key(),
            &accts.from_ata.key(),
            &accts.to_ata.key(),
            &accts.payer.key(),

            payment.pay_in_amount,
            0, //  min_out
        )?;

        invoke(
            &swap_ix,
            &[
                accts.token_program.to_account_info(),
                accts.amm_id.to_account_info(),
                accts.amm_authority.to_account_info(),
                accts.amm_open_orders.to_account_info(),
                accts.pool_coin_token_account.to_account_info(),
                accts.pool_pc_token_account.to_account_info(),
                accts.serum_program.to_account_info(),
                accts.serum_market.to_account_info(),
                accts.serum_bids.to_account_info(),
                accts.serum_asks.to_account_info(),
                accts.serum_event_queue.to_account_info(),
                accts.serum_coin_vault.to_account_info(),
                accts.serum_pc_vault.to_account_info(),
                accts.serum_vault_signer.to_account_info(),
                accts.from_ata.to_account_info(),
                accts.to_ata.to_account_info(),
                accts.payer.to_account_info(),
            ],
        )?;

        //  Get swap_out_amount
        accts.to_ata.reload()?;
        let to_ata_balance: u64 = accts.to_ata.amount;
        let swap_out_amount = to_ata_balance - initial_ata_balance;
        let fee_amount = swap_out_amount - payment.pay_out_amount;

        //  Transfer fee to treasury
        token::transfer(
            CpiContext::new(
                accts.token_program.to_account_info(),
                token::Transfer {
                    from: accts.to_ata.to_account_info(),
                    to: accts.treasury_ata.to_account_info(),
                    authority: accts.payer.to_account_info()
                },
            ),
            fee_amount
        )?;

        //  Transfer rest to merchant
        token::transfer(
            CpiContext::new(
                accts.token_program.to_account_info(),
                token::Transfer {
                    from: accts.to_ata.to_account_info(),
                    to: accts.merchant_ata.to_account_info(),
                    authority: accts.payer.to_account_info()
                },
            ),
            payment.pay_out_amount
        )?;

        // Emit an event after the successful payment
        emit!(SwapPaymentCompleted {
            order_id: payment.order_id,
            pay_in_token: payment.pay_in_token,
            pay_out_token: payment.pay_out_token,
            pay_in_amount: payment.pay_in_amount,
            pay_out_amount: payment.pay_out_amount,
            fee_collected: fee_amount,
            treasury: treasury_ata.key(),
            merchant: accts.merchant.key(),
            payer: payer.key()
        });

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

#[derive(Accounts)]
pub struct CompletePayment<'info> {
    #[account(mut)]
    payer: Signer<'info>,

    /// CHECK: token receiver
    #[account(mut)]
    merchant: AccountInfo<'info>,
    /// CHECK: treasury wallet address
    #[account(mut)]
    treasury: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: raydium will check
    amm_id: AccountInfo<'info>,
    /// CHECK: raydium will check
    amm_authority: AccountInfo<'info>,
    /// CHECK: raydium will check
    #[account(mut)]
    amm_open_orders: AccountInfo<'info>,

    #[account(mut)]
    pool_coin_token_account: Box<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    pool_pc_token_account: Box<Account<'info, TokenAccount>>,
  
    #[account(mut)]
    from_ata: Box<Account<'info, TokenAccount>>,
  
    #[account(mut)]
    to_ata: Box<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    treasury_ata: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    merchant_ata: Box<Account<'info, TokenAccount>>,
    
    #[account(mut)]
    /// CHECK: raydium will check
    amm_target_orders: AccountInfo<'info>,

    /// CHECK: raydium will check
    serum_program: AccountInfo<'info>,
    #[account(mut)]
    /// CHECK: raydium will check
    serum_market: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: raydium will check
    serum_bids: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: raydium will check
    serum_asks: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: raydium will check
    serum_event_queue: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: raydium will check
    serum_coin_vault: AccountInfo<'info>,

    #[account(mut)]
    /// CHECK: raydium will check
    serum_pc_vault: AccountInfo<'info>,

    /// CHECK: raydium will check
    serum_vault_signer: AccountInfo<'info>,

    /// CHECK: raydium will check
    raydium_amm_program: AccountInfo<'info>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

#[event]
pub struct PaymentCompleted {
    pub order_id: String,
    pub pay_in_token: Pubkey,
    pub pay_out_token: Pubkey,
    pub pay_in_amount: u64,
    pub pay_out_amount: u64,
    pub fee_collected: u64,
    pub treasury: Pubkey,
    pub merchant: Pubkey,
    pub payer: Pubkey,
}

#[event]
pub struct SwapPaymentCompleted {
    pub order_id: String,
    pub pay_in_token: Pubkey,
    pub pay_out_token: Pubkey,
    pub pay_in_amount: u64,
    pub pay_out_amount: u64,
    pub fee_collected: u64,
    pub treasury: Pubkey,
    pub merchant: Pubkey,
    pub payer: Pubkey,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The payment has expired.")]
    PaymentExpired,
}
