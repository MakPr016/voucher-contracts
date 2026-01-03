use anchor_lang::prelude::*;

declare_id!("8iRpzhFJF4PJnhyKZRDXk6B3TKjxQGEX6kcsteYq77iR");

#[program]
pub mod git_voucher_escrow {
    use super::*;

    pub fn initialize_organization(
        ctx: Context<InitializeOrganization>,
        org_github_id: u64,
    ) -> Result<()> {
        let org = &mut ctx.accounts.organization;
        org.org_github_id = org_github_id;
        org.admin = ctx.accounts.admin.key();
        org.balance = 0;
        org.maintainers = Vec::new();
        org.total_vouchers_created = 0;
        org.bump = ctx.bumps.organization;
        
        msg!("Organization escrow initialized for GitHub ID: {}", org_github_id);
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::ZeroAmount);

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.depositor.to_account_info(),
                    to: ctx.accounts.organization.to_account_info(),
                },
            ),
            amount,
        )?;

        let org = &mut ctx.accounts.organization;
        org.balance = org.balance.checked_add(amount).ok_or(ErrorCode::Overflow)?;

        msg!("Deposited {} lamports. New balance: {}", amount, org.balance);
        Ok(())
    }

    pub fn add_maintainer(ctx: Context<ManageMaintainer>, maintainer: Pubkey) -> Result<()> {
        let org = &mut ctx.accounts.organization;
        
        require!(!org.maintainers.contains(&maintainer), ErrorCode::MaintainerAlreadyExists);
        org.maintainers.push(maintainer);

        msg!("Maintainer added: {}", maintainer);
        Ok(())
    }

    pub fn remove_maintainer(ctx: Context<ManageMaintainer>, maintainer: Pubkey) -> Result<()> {
        let org = &mut ctx.accounts.organization;
        
        org.maintainers.retain(|&m| m != maintainer);

        msg!("Maintainer removed: {}", maintainer);
        Ok(())
    }

    pub fn create_voucher(
        ctx: Context<CreateVoucher>,
        voucher_id: String,
        recipient_github_id: u64,
        amount: u64,
        metadata: String,
    ) -> Result<()> {
        require!(amount > 0, ErrorCode::ZeroAmount);
        require!(voucher_id.len() <= 64, ErrorCode::VoucherIdTooLong);
        require!(metadata.len() <= 512, ErrorCode::MetadataTooLong);

        let org = &mut ctx.accounts.organization;
        require!(org.balance >= amount, ErrorCode::InsufficientBalance);
        require!(
            org.maintainers.contains(&ctx.accounts.maintainer.key()),
            ErrorCode::NotAuthorized
        );

        let org_key = org.key();

        let voucher = &mut ctx.accounts.voucher;
        voucher.voucher_id = voucher_id.clone();
        voucher.organization = org_key;
        voucher.recipient_github_id = recipient_github_id;
        voucher.amount = amount;
        voucher.created_at = Clock::get()?.unix_timestamp;
        voucher.expires_at = Clock::get()?.unix_timestamp + 30 * 24 * 60 * 60;
        voucher.state = VoucherState::Pending;
        voucher.metadata = metadata;
        voucher.bump = ctx.bumps.voucher;

        **ctx.accounts.organization.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.voucher.to_account_info().try_borrow_mut_lamports()? += amount;

        let org = &mut ctx.accounts.organization;
        org.balance = org.balance.checked_sub(amount).ok_or(ErrorCode::Underflow)?;
        org.total_vouchers_created += 1;

        msg!("Voucher created: {} for recipient GitHub ID: {}", voucher_id, recipient_github_id);
        Ok(())
    }

    pub fn claim_voucher(ctx: Context<ClaimVoucher>) -> Result<()> {
        let voucher = &ctx.accounts.voucher;
        
        require!(voucher.state == VoucherState::Pending, ErrorCode::InvalidVoucherState);
        require!(Clock::get()?.unix_timestamp <= voucher.expires_at, ErrorCode::VoucherExpired);

        let amount = voucher.amount;

        **ctx.accounts.voucher.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.recipient.to_account_info().try_borrow_mut_lamports()? += amount;

        let voucher_mut = &mut ctx.accounts.voucher;
        voucher_mut.state = VoucherState::Claimed;

        msg!("Voucher claimed by: {}", ctx.accounts.recipient.key());
        Ok(())
    }

    pub fn cancel_voucher(ctx: Context<CancelVoucher>) -> Result<()> {
        let voucher = &ctx.accounts.voucher;
        let org = &ctx.accounts.organization;

        require!(voucher.state == VoucherState::Pending, ErrorCode::InvalidVoucherState);
        require!(
            org.maintainers.contains(&ctx.accounts.maintainer.key()),
            ErrorCode::NotAuthorized
        );

        let amount = voucher.amount;
        let voucher_id = voucher.voucher_id.clone();

        **ctx.accounts.voucher.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.organization.to_account_info().try_borrow_mut_lamports()? += amount;

        let voucher_mut = &mut ctx.accounts.voucher;
        voucher_mut.state = VoucherState::Cancelled;

        let org_mut = &mut ctx.accounts.organization;
        org_mut.balance = org_mut.balance.checked_add(amount).ok_or(ErrorCode::Overflow)?;

        msg!("Voucher cancelled: {}", voucher_id);
        Ok(())
    }

    pub fn expire_voucher(ctx: Context<ExpireVoucher>) -> Result<()> {
        let voucher = &ctx.accounts.voucher;

        require!(voucher.state == VoucherState::Pending, ErrorCode::InvalidVoucherState);
        require!(Clock::get()?.unix_timestamp > voucher.expires_at, ErrorCode::VoucherNotExpired);

        let amount = voucher.amount;
        let voucher_id = voucher.voucher_id.clone();

        **ctx.accounts.voucher.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.organization.to_account_info().try_borrow_mut_lamports()? += amount;

        let voucher_mut = &mut ctx.accounts.voucher;
        voucher_mut.state = VoucherState::Expired;

        let org_mut = &mut ctx.accounts.organization;
        org_mut.balance = org_mut.balance.checked_add(amount).ok_or(ErrorCode::Overflow)?;

        msg!("Voucher expired: {}", voucher_id);
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::ZeroAmount);

        let org = &ctx.accounts.organization;
        require!(org.balance >= amount, ErrorCode::InsufficientBalance);

        **ctx.accounts.organization.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.admin.to_account_info().try_borrow_mut_lamports()? += amount;

        let org_mut = &mut ctx.accounts.organization;
        org_mut.balance = org_mut.balance.checked_sub(amount).ok_or(ErrorCode::Underflow)?;

        msg!("Withdrawn {} lamports. New balance: {}", amount, org_mut.balance);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(org_github_id: u64)]
pub struct InitializeOrganization<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + OrganizationEscrow::INIT_SPACE,
        seeds = [b"organization", org_github_id.to_le_bytes().as_ref()],
        bump
    )]
    pub organization: Account<'info, OrganizationEscrow>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub organization: Account<'info, OrganizationEscrow>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ManageMaintainer<'info> {
    #[account(
        mut,
        has_one = admin
    )]
    pub organization: Account<'info, OrganizationEscrow>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(voucher_id: String)]
pub struct CreateVoucher<'info> {
    #[account(mut)]
    pub organization: Account<'info, OrganizationEscrow>,
    #[account(
        init,
        payer = maintainer,
        space = 8 + VoucherEscrow::INIT_SPACE,
        seeds = [b"voucher", voucher_id.as_bytes()],
        bump
    )]
    pub voucher: Account<'info, VoucherEscrow>,
    #[account(mut)]
    pub maintainer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimVoucher<'info> {
    #[account(mut)]
    pub voucher: Account<'info, VoucherEscrow>,
    #[account(mut)]
    pub recipient: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelVoucher<'info> {
    #[account(mut)]
    pub organization: Account<'info, OrganizationEscrow>,
    #[account(mut)]
    pub voucher: Account<'info, VoucherEscrow>,
    pub maintainer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExpireVoucher<'info> {
    #[account(mut)]
    pub organization: Account<'info, OrganizationEscrow>,
    #[account(mut)]
    pub voucher: Account<'info, VoucherEscrow>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        has_one = admin
    )]
    pub organization: Account<'info, OrganizationEscrow>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct OrganizationEscrow {
    pub org_github_id: u64,
    pub admin: Pubkey,
    pub balance: u64,
    #[max_len(10)]
    pub maintainers: Vec<Pubkey>,
    pub total_vouchers_created: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct VoucherEscrow {
    #[max_len(64)]
    pub voucher_id: String,
    pub organization: Pubkey,
    pub recipient_github_id: u64,
    pub amount: u64,
    pub created_at: i64,
    pub expires_at: i64,
    pub state: VoucherState,
    #[max_len(512)]
    pub metadata: String,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum VoucherState {
    Pending,
    Claimed,
    Cancelled,
    Expired,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Not authorized")]
    NotAuthorized,
    #[msg("Maintainer already exists")]
    MaintainerAlreadyExists,
    #[msg("Voucher ID too long (max 64 characters)")]
    VoucherIdTooLong,
    #[msg("Metadata too long (max 512 characters)")]
    MetadataTooLong,
    #[msg("Invalid voucher state")]
    InvalidVoucherState,
    #[msg("Voucher has expired")]
    VoucherExpired,
    #[msg("Voucher has not expired yet")]
    VoucherNotExpired,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Arithmetic underflow")]
    Underflow,
}
