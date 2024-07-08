use anchor_lang::prelude::*;
declare_id!("FxouSLUnFdHhvAz1HVfSwHDxJyhomf9miwh2YNW7FAHR");
use anchor_spl::token::*;

#[program]
mod tokenvest_campaign {
    use super::*;
    pub fn initialize(
        ctx: Context<Initialize>,
        campaign_seed: String,
        investment_goal: u64,
        end_time: i64,
    ) -> Result<()> {
        let investment_contract = &mut ctx.accounts.investment_contract;
        let startup_owner = &mut ctx.accounts.startup_owner;
        investment_contract.startup_owner = *startup_owner.key;
        investment_contract.investment_goal = investment_goal;
        investment_contract.campaign_seed = campaign_seed;
        investment_contract.start_time = ctx.accounts.clock.unix_timestamp;
        investment_contract.end_time = end_time;
        investment_contract.bump = ctx.bumps.investment_contract;
        investment_contract.usdc_vault = ctx.accounts.usdc_vault.key();
        msg!("first goal is: {}!", investment_goal);
        msg!("start time is: {}!", investment_contract.start_time);
        msg!("goal is: {}!", investment_contract.investment_goal);
        msg!("end_time is: {}!", end_time);
        Ok(())
    }

    pub fn invest(ctx: Context<Invest>, investment_amount: u64) -> Result<()> {
        let from_account = &ctx.accounts.from;
        let investment_contract = &mut ctx.accounts.investment_contract;
        let cpi_accounts = Transfer {
            from: ctx.accounts.investor_ata.to_account_info(),
            to: ctx.accounts.usdc_vault.to_account_info(),
            authority: from_account.to_account_info(),
        };

        transfer(
            CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts),
            investment_amount,
        )?;
        if ctx.accounts.investor_data.amount == None {
            ctx.accounts.investor_data.set_inner(InvestorData {
                pubkey: from_account.key(),
                amount: Some(investment_amount),
            });

            Ok(())
        } else {
            ctx.accounts.investor_data.amount =
                Some(ctx.accounts.investor_data.amount.unwrap() + investment_amount);
            Ok(())
        }
    }

    pub fn finish_startup(ctx: Context<FinishStartup>) -> Result<()> {
        let investment_contract = &mut ctx.accounts.investment_contract;
        let startup_owner = investment_contract.startup_owner;
        let caller = &ctx.accounts.caller;

        if investment_contract.end_time > ctx.accounts.clock.unix_timestamp {
            msg!("CAMPAIGN STILL RUNNING");
            Ok(())
        } else {
            msg!("goal is: {}!", investment_contract.investment_goal);
            msg!(
                "coll amount  is: {}!",
                ctx.accounts.usdc_vault.amount.to_string()
            );
            if ctx.accounts.usdc_vault.amount < investment_contract.investment_goal {
                msg!("CAMPAIGN FAILED");
                Ok(())
            } else {
                let final_amount = ctx.accounts.usdc_vault.amount;
                if caller.key == &startup_owner {
                    let cpi_accounts = Transfer {
                        from: ctx.accounts.usdc_vault.to_account_info(),
                        to: ctx.accounts.caller_ata.to_account_info(),
                        authority: investment_contract.to_account_info(),
                    };

                    let investment_contract_signer_seeds: &[&[&[u8]]] = &[&[
                        b"tokenvest",
                        investment_contract.startup_owner.as_ref(),
                        investment_contract.campaign_seed.as_str().as_bytes(),
                        &[investment_contract.bump],
                    ]];

                    transfer(
                        CpiContext::new_with_signer(
                            ctx.accounts.token_program.to_account_info(),
                            cpi_accounts,
                            investment_contract_signer_seeds,
                        ),
                        final_amount,
                    )
                } else {
                    msg!("Unknown Caller: Cannot Withdraw Funds");
                    Ok(())
                }
            }
        }
    }

    pub fn refund_startup(ctx: Context<RefundStartup>) -> Result<()> {
        let investment_contract = &mut ctx.accounts.investment_contract;

        if investment_contract.end_time > ctx.accounts.clock.unix_timestamp {
            msg!("CAMPAIGN STILL RUNNING");
            Ok(())
        } else {
            if ctx.accounts.usdc_vault.amount > investment_contract.investment_goal {
                msg!("CAMPAING FINISHED SUCCESFULLY");
                Ok(())
            } else {
                if ctx.accounts.investor_data.amount == None {
                    msg!("no backing history");
                    Ok(())
                } else {
                    let cpi_accounts = Transfer {
                        from: ctx.accounts.usdc_vault.to_account_info(),
                        to: ctx.accounts.caller_ata.to_account_info(),
                        authority: investment_contract.to_account_info(),
                    };

                    let investment_contract_signer_seeds: &[&[&[u8]]] = &[&[
                        b"tokenvest",
                        investment_contract.startup_owner.as_ref(),
                        investment_contract.campaign_seed.as_str().as_bytes(),
                        &[investment_contract.bump],
                    ]];

                    transfer(
                        CpiContext::new_with_signer(
                            ctx.accounts.token_program.to_account_info(),
                            cpi_accounts,
                            investment_contract_signer_seeds,
                        ),
                        ctx.accounts.investor_data.amount.unwrap(),
                    )
                }
            }
        }
    }
}

#[derive(Accounts)]
#[instruction(campaign_seed: String,
    investment_goal: u64,
    end_time: i64)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub startup_owner: Signer<'info>,
    #[account(
        init,
        payer = startup_owner,
        seeds = ["tokenvest".as_bytes(),
        startup_owner.key().as_ref(),
        campaign_seed.as_str().as_bytes()],
        space = 8 + InvestmentContract::SIZE,
        bump
    )]
    pub investment_contract: Account<'info, InvestmentContract>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(
         init,
         payer = startup_owner,
         seeds = ["tokenvest".as_bytes(), investment_contract.key().as_ref()],
         token::mint = usdc_mint,
         token::authority = investment_contract,
         bump,
     )]
    pub usdc_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct Invest<'info> {
    #[account(
        mut,
        seeds = ["tokenvest".as_bytes(),
        investment_contract.startup_owner.as_ref(),
        investment_contract.campaign_seed.as_str().as_bytes()],
        bump=investment_contract.bump
    )]
    pub investment_contract: Account<'info, InvestmentContract>,
    #[account(
        init_if_needed,
        payer = from,
        seeds = ["tokenvest".as_bytes(),
        from.key().as_ref(),
        investment_contract.campaign_seed.as_str().as_bytes()],
        space = 8 + InvestorData::SIZE,
        bump,
    )]
    pub investor_data: Account<'info, InvestorData>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(mut)]
    pub from: Signer<'info>,
    #[account(mut,
        constraint = investor_ata.mint == usdc_mint.key() &&
        investor_ata.owner == from.key())]
    pub investor_ata: Account<'info, TokenAccount>,
    #[account(mut, constraint = usdc_vault.key() == investment_contract.usdc_vault)]
    pub usdc_vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinishStartup<'info> {
    #[account(
        mut,
        seeds = ["tokenvest".as_bytes(),
        investment_contract.startup_owner.as_ref(),
        investment_contract.campaign_seed.as_str().as_bytes()],
        bump=investment_contract.bump
    )]
    pub investment_contract: Account<'info, InvestmentContract>,
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(mut,
        constraint = caller_ata.mint == usdc_mint.key() &&
        caller_ata.owner == caller.key())]
    pub caller_ata: Account<'info, TokenAccount>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(mut, constraint = usdc_vault.key() == investment_contract.usdc_vault)]
    pub usdc_vault: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct RefundStartup<'info> {
    #[account(
        mut,
        seeds = ["tokenvest".as_bytes(),
        investment_contract.startup_owner.as_ref(),
        investment_contract.campaign_seed.as_str().as_bytes()],
        bump
    )]
    pub investment_contract: Account<'info, InvestmentContract>,
    #[account(
        mut,
        seeds = ["tokenvest".as_bytes(),
        caller.key().as_ref(),
        investment_contract.campaign_seed.as_str().as_bytes()],
        bump
    )]
    pub investor_data: Account<'info, InvestorData>,
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(mut,
        constraint = caller_ata.mint == usdc_mint.key() &&
        caller_ata.owner == caller.key())]
    pub caller_ata: Account<'info, TokenAccount>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(mut, constraint = usdc_vault.key() == investment_contract.usdc_vault)]
    pub usdc_vault: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[account]
pub struct InvestmentContract {
    pub startup_owner: Pubkey,
    pub start_time: i64,
    pub usdc_vault: Pubkey,
    pub end_time: i64,
    pub tokens_collected: u64,
    pub investment_goal: u64,
    pub campaign_seed: String,
    pub bump: u8,
}

impl InvestmentContract {
    const SIZE: usize = 32 + 8 + 32 + 8 + 8 + 8 + 32 + 1;
}

#[account]
pub struct InvestorData {
    pubkey: Pubkey,
    amount: Option<u64>,
}

impl InvestorData {
    const SIZE: usize = 32 + 16;
}
