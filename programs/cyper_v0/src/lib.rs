pub mod constants;

use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{self,Mint, TokenAccount, TokenInterface, TransferChecked};

declare_id!("EFJgwKYkTazkeeHxSh2XyeMwxAtBhKqVATTdqrNzRKBk");

#[program]
pub mod cyper_v0 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee: u16, creator_bond: u64) -> Result<()> {
        initialize_handler(ctx, fee, creator_bond)
    }

    pub fn create_market(
        ctx: Context<CreateMarket>,
        question_text: String,
        fixed_price: u64,
        market_type: MarketType,
        category: MarketCategory,
        lp_fee_bps: Option<u16>,
        resolution_deadline: i64,
        market_group: Option<Pubkey>,
        market_data: MarketData,
    ) -> Result<()> {
        create_market_handler(
            ctx,
            question_text,
            fixed_price,
            market_type,
            category,
            lp_fee_bps,
            resolution_deadline,
            market_group,
            market_data,
        )
    }

    pub fn place_bet(ctx: Context<PlaceBet>, amount: u64, bet_data: BetData) -> Result<()> {
        place_bet_handler(ctx, amount, bet_data)
    }

    pub fn lock_market(ctx: Context<LockMarket>) -> Result<()> {
        lock_market_handler(ctx)
    }

    pub fn settle_market(ctx: Context<SettleMarket>, outcome: WinningOutcome) -> Result<()> {
        settle_market_handler(ctx, outcome)
    }

    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        claim_handler(ctx)
    }

    pub fn set_accuracy_payout(ctx: Context<SetAccuracyPayout>, amount: u64) -> Result<()> {
        set_accuracy_payout_handler(ctx, amount)
    }
}

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub bettor: Signer<'info>,

    #[account(
        mut,
        seeds = [b"bet", market.key().as_ref(), bet.bet_index.to_le_bytes().as_ref()],
        bump = bet.bump,
        has_one = bettor @ ErrorCode::UnauthorizedAction,
        has_one = market @ ErrorCode::InvalidBetAccount,
        close = bettor 
    )]
    pub bet: Account<'info, Bet>,

    #[account(
        mut,
        seeds = [b"market", market.market_index.to_le_bytes().as_ref()],
        bump = market.bump,
        constraint = market.status == MarketStatus::Settled || market.status == MarketStatus::Voided @ ErrorCode::MarketNotSettled
    )]
    pub market: Account<'info, Market>,

    #[account(address = market.token_mint @ ErrorCode::InvalidMint)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = market,
        associated_token::token_program = token_program
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = bettor,
        associated_token::mint = mint,
        associated_token::authority = bettor,
        associated_token::token_program = token_program
    )]
    pub bettor_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetAccuracyPayout<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        seeds = [b"market", market.market_index.to_le_bytes().as_ref()],
        bump = market.bump,
        has_one = creator @ ErrorCode::UnauthorizedAction,
    )]
    pub market: Account<'info, Market>,

    #[account(
        mut,
        has_one = market,
    )]
    pub bet: Account<'info, Bet>,
}

#[derive(InitSpace)]
#[account]
pub struct CyperMarket {
    pub bump: u8,
    pub authority: Pubkey,             // admin wallet — THE top-level authority
    pub treasury: Pubkey,              // fee collection wallet
    pub default_protocol_fee_bps: u16, // e.g. 50 = 0.50%
    pub default_creator_bond: u64,
    pub market_count: u64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    // Stable Coin Ata for Treasury
    #[account(
        init,
        payer = authority,
        associated_token::mint = mint,
        associated_token::authority = authority,
        associated_token::token_program = token_program
    )]
    pub treasury: InterfaceAccount<'info, TokenAccount>,

   #[account(
    init,
        payer = authority,
        space = 8 + CyperMarket::INIT_SPACE, 
        seeds = [b"protocol"],
        bump
    )]
    pub market: Account<'info, CyperMarket>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn initialize_handler(ctx: Context<Initialize>, fee: u16, creator_bond: u64) -> Result<()> {
    msg!("Greetings from: {:?}", ctx.program_id);
    let market = &mut ctx.accounts.market;
    market.bump = ctx.bumps.market;
    market.authority = ctx.accounts.authority.key();
    market.treasury = ctx.accounts.treasury.key();
    market.default_protocol_fee_bps = fee;
    market.default_creator_bond = creator_bond;
    market.market_count = 0;
    Ok(())
}

#[derive(InitSpace)]
#[account]
pub struct Market {
    pub bump: u8,
    pub creator: Pubkey, // creator wallet — can lock, settle, withdraw bond+fees
    // public key of the market authority wallet
    // no of markets created by global cyper 
    pub market_index: u64,
    pub user_bet_index: u64,
    pub market_type: MarketType,  // YesNo | MultiOutcome | Accuracy
    pub category: MarketCategory, // Crypto | Politics | Sports | Tech | Economy | Culture | Beyond
    pub status: MarketStatus,     // Open | Locked | Settled | Voided
    #[max_len(200)]
    pub question: String, // max 200 bytes
    pub creator_bond: u64,
    pub lp_fee_bps: Option<u16>,
    pub protocol_fee_bps: u16,
    pub resolution_deadline: i64,
    pub created_at: i64,
    pub total_bets: u64,
    pub vault: Pubkey,
    pub token_mint: Pubkey,
    pub market_group: Option<Pubkey>, // None for standalone, Some for tiered
    pub total_liquidity: u64,
    pub market_data: MarketData,
    pub winning_outcome: Option<WinningOutcome>,
}


#[derive(Accounts)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub market_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"protocol"],
        bump = cyper_market.bump,
    )]
    pub cyper_market: Account<'info, CyperMarket>,

    // User can make any amount of markets on same question but on different prices, time, market_index so seeding with market_index is better
    #[account(init,
        payer = market_authority,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market", cyper_market.market_count.to_le_bytes().as_ref()],
        bump,
    )]
    pub market: Account<'info, Market>,

    pub mint: InterfaceAccount<'info, Mint>,

    // Stable Coin Ata for Treasury
    #[account(
        init,
        payer = market_authority,
        associated_token::mint = mint,
        associated_token::authority = market,
        associated_token::token_program = token_program
    )]
    pub market_vault_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = market_authority,
        associated_token::token_program = token_program
    )]
    pub market_authority_ata: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn create_market_handler(
    ctx: Context<CreateMarket>,
    question_text: String,
    _fixed_price: u64,
    market_type: MarketType,
    category: MarketCategory,
    lp_fee_bps: Option<u16>,
    resolution_deadline: i64,
    market_group: Option<Pubkey>,
    market_data: MarketData,
) -> Result<()> {

    require!(question_text.len() <= 200, ErrorCode::QuestionTooLong);

    let new_market = &mut ctx.accounts.market;
    let cyper_market = &mut ctx.accounts.cyper_market;

    new_market.creator = ctx.accounts.market_authority.key();
    new_market.market_index = cyper_market.market_count;

    new_market.bump = ctx.bumps.market;
    new_market.category = category;
    new_market.status = MarketStatus::Open;
    new_market.question = question_text;

    // Initialize MarketData based on MarketType
    new_market.market_data = match market_type {
        MarketType::YesNo => MarketData::YesNo { yes_pool: 0, no_pool: 0 },
        MarketType::MultiOutcome { outcome_count } => {
            require!(outcome_count >= 2 && outcome_count <= 10, ErrorCode::InvalidPrice);
            MarketData::MultiOutcome { pools: vec![0; outcome_count as usize] }
        },
        MarketType::Accuracy { .. } => MarketData::Accuracy { total_pool: 0 },
    };

    if new_market.creator == cyper_market.authority {
        new_market.creator_bond = 0;
    } else {
        new_market.creator_bond = cyper_market.default_creator_bond;
    }

    // Enforce economic rules based on the market type
    match market_type {
        MarketType::Accuracy { fixed_price: _ } => {
            // Accuracy markets use parimutuel entry fees, NOT liquidity pools.
            require!(lp_fee_bps.unwrap_or(0) == 0, ErrorCode::NoLpsInAccuracyMarkets);
        },
        MarketType::YesNo | MarketType::MultiOutcome { .. } => {
            // Standard AMM markets require an LP fee to incentivize liquidity providers
            require!(lp_fee_bps.unwrap_or(0) > 0, ErrorCode::LpFeeRequired);
        }
    }

    new_market.market_type = market_type;
    new_market.lp_fee_bps = lp_fee_bps;
    
    // Set protocol fee: Admin full fee go to treasury, everyone else pays half fee
    if new_market.creator == cyper_market.authority {
        new_market.protocol_fee_bps = cyper_market.default_protocol_fee_bps
    } else {
        new_market.protocol_fee_bps = cyper_market.default_protocol_fee_bps.checked_div(2).unwrap_or(0);
    }
    
    new_market.resolution_deadline = resolution_deadline;
    new_market.created_at = Clock::get()?.unix_timestamp;
    new_market.total_bets = 0;
   
    new_market.token_mint = ctx.accounts.mint.key();
    new_market.market_group = market_group;
    new_market.market_data = market_data;
    new_market.user_bet_index = 0;

    // CPI: Transfer creator bond
    if new_market.creator_bond > 0 {
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.market_authority_ata.to_account_info(),
            to: ctx.accounts.market_vault_ata.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            authority: ctx.accounts.market_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);
        token_interface::transfer_checked(cpi_ctx, new_market.creator_bond, ctx.accounts.mint.decimals)?;
    }

    cyper_market.market_count = cyper_market
        .market_count
        .checked_add(1)
        .ok_or(ErrorCode::MarketCountOverflow)?;
    Ok(())
}

#[derive(InitSpace)]
#[account]
pub struct Bet {
    pub bump: u8,
    pub bettor: Pubkey,
    pub market: Pubkey,
    pub bet_index: u64,
    pub created_at: i64,
    pub claimed: bool,
    pub amount: u64,
    pub bet_data: BetData,
    pub payout_amount: Option<u64>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub better: Signer<'info>,

    #[account(address = market.token_mint @ErrorCode::InvalidMint)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut,
    seeds=[
        b"market", market.market_index.to_le_bytes().as_ref()
    ],
    bump=market.bump
    )]
    pub market: Account<'info, Market>,

    #[account(init,
    payer = better,
    space = 8 + Bet::INIT_SPACE,
    seeds = [b"bet", market.key().as_ref(), market.user_bet_index.to_le_bytes().as_ref()],
    bump,
    )]
    pub bet: Account<'info, Bet>,

    #[account(
        init_if_needed,
        payer = better,
        associated_token::mint = mint,
        associated_token::authority = better,
        associated_token::token_program = token_program
    )]
    pub better_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(mut,
   associated_token::mint = mint,
   associated_token::authority = market,
   associated_token::token_program = token_program
    )]
    pub market_vault: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn place_bet_handler(ctx: Context<PlaceBet>, amount: u64, bet_data: BetData) -> Result<()> {
    let market = &mut ctx.accounts.market;
    
    require!(market.status == MarketStatus::Open, ErrorCode::MarketNotOpen);
    require!(Clock::get()?.unix_timestamp < market.resolution_deadline, ErrorCode::InvalidBettingWindow);

    match (&market.market_type, &bet_data) {
        (MarketType::Accuracy { fixed_price }, BetData::Accuracy { predicted_value: _ }) => {
            require!(amount == *fixed_price, ErrorCode::InvalidBetAmount);
        },
        (MarketType::YesNo, BetData::YesNo { direction: _ }) => {
            require!(amount > 0, ErrorCode::InvalidBetAmount);
        },
        (MarketType::MultiOutcome{outcome_count: _}, BetData::MultiOutcome { outcome_index: _ }) => {
            require!(amount > 0, ErrorCode::InvalidBetAmount);
        },
        _ => return Err(ErrorCode::MarketTypeMismatch.into()),
    }

    let bet = &mut ctx.accounts.bet;
    bet.bump = ctx.bumps.bet;
    bet.bettor = ctx.accounts.better.key();
    bet.market = market.key();
    bet.bet_index = market.user_bet_index;
    bet.created_at = Clock::get()?.unix_timestamp;
    bet.claimed = false;
    bet.amount = amount;

    
    
    bet.bet_data = bet_data;
    market.user_bet_index = market.user_bet_index.checked_add(1).ok_or(ErrorCode::MarketCountOverflow)?;
    market.total_bets = market.total_bets.checked_add(1).ok_or(ErrorCode::MarketCountOverflow)?;
    market.total_liquidity = market.total_liquidity.checked_add(amount).ok_or(ErrorCode::MarketCountOverflow)?;

    match (&mut market.market_data, &bet.bet_data) {
        (MarketData::Accuracy { total_pool }, BetData::Accuracy { .. }) => {
            *total_pool = total_pool.checked_add(amount).ok_or(ErrorCode::MarketCountOverflow)?;
        },
        (MarketData::YesNo { yes_pool, no_pool }, BetData::YesNo { direction }) => {
            if *direction {
                *yes_pool = yes_pool.checked_add(amount).ok_or(ErrorCode::MarketCountOverflow)?;
            } else {
                *no_pool = no_pool.checked_add(amount).ok_or(ErrorCode::MarketCountOverflow)?;
            }
        },
        (MarketData::MultiOutcome { pools }, BetData::MultiOutcome { outcome_index }) => {
            let idx = *outcome_index as usize;
            require!(idx < pools.len(), ErrorCode::MarketTypeMismatch);
            pools[idx] = pools[idx].checked_add(amount).ok_or(ErrorCode::MarketCountOverflow)?;
        },
        _ => return Err(ErrorCode::MarketTypeMismatch.into()),
    }

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.better_vault.to_account_info(),
        to: ctx.accounts.market_vault.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        authority: ctx.accounts.better.to_account_info(), 
    }; 
   
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);
    token_interface::transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;

    Ok(())
}

pub fn claim_handler(ctx: Context<Claim>) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let bet = &mut ctx.accounts.bet;

    // 1. Prevent double-claiming
    require!(!bet.claimed, ErrorCode::AlreadyClaimed);

    let mut payout: u64 = 0;

    // 2. ROUTE BY STATUS: Voided vs Settled
    if market.status == MarketStatus::Voided {
        // REFUND: If the market is voided, refund their exact original bet amount.
        payout = bet.amount;
    } else if market.status == MarketStatus::Settled {
        let winning_outcome = market.winning_outcome.as_ref().ok_or(ErrorCode::MarketNotSettled)?;

        // 3. PARIMUTUEL MATH & PAYOUT ROUTER
        match (&market.market_type, winning_outcome, &bet.bet_data) {
            // A. ACCURACY MARKETS (TREPA MODEL)
            (MarketType::Accuracy { .. }, WinningOutcome::Accuracy { .. }, BetData::Accuracy { .. }) => {
                // Payout was calculated off-chain and injected by the crank
                payout = bet.payout_amount.ok_or(ErrorCode::PayoutNotSet)?; 
            },
            
            // B. YES/NO BINARY MARKETS
            (MarketType::YesNo, WinningOutcome::YesNo { winning_direction }, BetData::YesNo { direction }) => {
                if winning_direction == direction { 
                    if let MarketData::YesNo { yes_pool, no_pool } = market.market_data {
                        let winning_pool = if *winning_direction { yes_pool } else { no_pool };
                        
                        if winning_pool > 0 {
                            // Safe Math: (User_Bet_Amount * Total_Market_Liquidity) / Winning_Pool
                            payout = (bet.amount as u128)
                                .checked_mul(market.total_liquidity as u128)
                                .ok_or(ErrorCode::MathOverflow)? 
                                .checked_div(winning_pool as u128)
                                .ok_or(ErrorCode::MathOverflow)? as u64;
                        }
                    }
                }
            },
            
            // C. MULTI-OUTCOME MARKETS (SPORTS / ELECTIONS)
            (MarketType::MultiOutcome { .. }, WinningOutcome::MultiOutcome { winning_index }, BetData::MultiOutcome { outcome_index }) => {
                if winning_index == outcome_index { 
                    if let MarketData::MultiOutcome { pools } = &market.market_data {
                        let winning_pool = pools[*winning_index as usize];
                        
                        if winning_pool > 0 {
                             // Safe Math: (User_Bet_Amount * Total_Market_Liquidity) / Winning_Pool
                            payout = (bet.amount as u128)
                                .checked_mul(market.total_liquidity as u128)
                                .ok_or(ErrorCode::MathOverflow)? 
                                .checked_div(winning_pool as u128)
                                .ok_or(ErrorCode::MathOverflow)? as u64;
                        }
                    }
                }
            },
            
            _ => return Err(ErrorCode::MarketTypeMismatch.into()),
        }
    } else {
        return Err(ErrorCode::MarketNotSettled.into());
    }

    // 4. ZERO-PAYOUT REVERT
    require!(payout > 0, ErrorCode::NothingToClaim); 

    // 5. EFFECTS
    bet.claimed = true; 

    // 6. CPI TRANSFER WITH PDA SIGNER
    let seeds = &[
        b"market".as_ref(),
        &market.market_index.to_le_bytes(),
        &[market.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.market_vault.to_account_info(),
        to: ctx.accounts.bettor_vault.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        authority: market.to_account_info(), 
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(), 
        cpi_accounts, 
        signer_seeds 
    );

    anchor_spl::token_interface::transfer_checked(
        cpi_ctx, 
        payout, 
        ctx.accounts.mint.decimals
    )?;

    msg!("Successfully claimed {} tokens! PDA closed and rent refunded.", payout);

    Ok(())
}

pub fn set_accuracy_payout_handler(ctx: Context<SetAccuracyPayout>, amount: u64) -> Result<()> {
    let market = &ctx.accounts.market;
    let bet = &mut ctx.accounts.bet;

    require!(market.status == MarketStatus::Settled, ErrorCode::MarketNotSettled);
    require!(matches!(market.market_type, MarketType::Accuracy { .. }), ErrorCode::MarketTypeMismatch);

    bet.payout_amount = Some(amount);
    Ok(())
}






#[derive(Accounts)]
pub struct LockMarket<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"market", market.market_index.to_le_bytes().as_ref()],
        bump = market.bump,
        has_one = creator @ ErrorCode::UnauthorizedAction 
    )]
    pub market: Account<'info, Market>,
}


pub fn lock_market_handler(ctx: Context<LockMarket>) -> Result<()> {
    let market = &mut ctx.accounts.market;

    // Ensure the market is currently open
    require!(market.status == MarketStatus::Open, ErrorCode::MarketNotOpen);

    // Ensure the resolution deadline has actually passed
    let current_time = Clock::get()?.unix_timestamp;
    require!(current_time >= market.resolution_deadline, ErrorCode::ResolutionTooEarly);

    // Update the status
    market.status = MarketStatus::Locked;

    msg!("Market {} is now Locked. No more bets can be placed!", market.market_index);
    Ok(())
}


#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum BetData {
    // Stores the user's prediction value
    Accuracy { predicted_value: u64 }, 
    // Stores true for "Yes", false for "No"
    YesNo { direction: bool },         
    // Stores the index of the outcome they are betting on
    MultiOutcome { outcome_index: u8 }, 
}

#[derive(Accounts)]
pub struct SettleMarket<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"market", market.market_index.to_le_bytes().as_ref()],
        bump = market.bump,
        has_one = creator @ ErrorCode::UnauthorizedAction
    )]
    pub market: Account<'info, Market>,
}

pub fn settle_market_handler(ctx: Context<SettleMarket>, outcome: WinningOutcome) -> Result<()> {
    let market = &mut ctx.accounts.market;

    // 1. STATE CHECK: Market must be locked before settlement
    require!(market.status == MarketStatus::Locked, ErrorCode::MarketNotLocked);

    // 2. SECURITY CHECK: Ensure outcome type matches market type
    match (&market.market_type, &outcome) {
        (MarketType::Accuracy { .. }, WinningOutcome::Accuracy { .. }) => {},
        (MarketType::YesNo, WinningOutcome::YesNo { .. }) => {},
        (MarketType::MultiOutcome{..}, WinningOutcome::MultiOutcome { .. }) => {},
        _ => return Err(ErrorCode::MarketTypeMismatch.into()),
    }

    // 3. EFFECT: Store the outcome and update status
    market.winning_outcome = Some(outcome);
    market.status = MarketStatus::Settled;

    msg!("Market {} successfully settled!", market.market_index);
    Ok(())
}


#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum MarketType {
    YesNo,
    MultiOutcome { outcome_count: u8 },
    Accuracy { fixed_price: u64 },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum MarketCategory {
    Crypto,
    Politics,
    Sports,
    Tech,
    Economy,
    Culture,
    Beyond,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum MarketStatus {
    Open,
    Locked,
    Settled,
    Voided,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum MarketData {
    YesNo { 
        yes_pool: u64, 
        no_pool: u64 
    },
    MultiOutcome { 
        #[max_len(10)]
        pools: Vec<u64> 
    },
    Accuracy { 
        total_pool: u64 
    },
}


#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum WinningOutcome {
    Accuracy { actual_value: u64 },
    YesNo { winning_direction: bool },
    MultiOutcome { winning_index: u8 },
}



#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Fee")]
    InvalidFee,
    #[msg("Max Market Reached")]
    MaxMarketReached,
    #[msg("Invalid Deadline")]
    InvalidDeadline,
    #[msg("Not Authorized")]
    NotAuthorized,
    #[msg("Invalid Question Text")]
    InvalidQuestionText,
    #[msg("Invalid Price")]
    InvalidPrice,
    #[msg("Market already exists")]
    MarketAlreadyExists,
    #[msg("Market already settled")]
    MarketAlreadySettled,
    #[msg("Market already voided")]
    MarketAlreadyVoided,
    #[msg("Market not settled")]
    MarketNotSettled,
    #[msg("Market not voided")]
    MarketNotVoided,
    #[msg("Market not open")]
    MarketNotOpen,
    #[msg("Market not locked")]
    MarketNotLocked,
    #[msg("Market count overflow")]
    MarketCountOverflow,
    #[msg("Accuracy markets do not have Liquidity Providers. LP fees must be 0.")]
    NoLpsInAccuracyMarkets,
    #[msg("Standard markets require an LP fee to function.")]
    LpFeeRequired,
    #[msg("Question length exceeds 200 bytes limit.")]
    QuestionTooLong,
    #[msg("Invalid betting window")]
    InvalidBettingWindow,
    #[msg("Invalid token mint")]
    InvalidMint,
    #[msg("Invalid bet amount")]
    InvalidBetAmount,
    #[msg("Market type mismatch")]
    MarketTypeMismatch,
    #[msg("Unauthorized action")]
    UnauthorizedAction,
    #[msg("Resolution window has not been reached yet.")]
    ResolutionTooEarly,
    #[msg("Bet has already been claimed.")]
    AlreadyClaimed,
    #[msg("Invalid bet account for this market.")]
    InvalidBetAccount,
    #[msg("Payout has not been set for this accuracy market yet.")]
    PayoutNotSet,
    #[msg("Mathematical overflow during calculation.")]
    MathOverflow,
    #[msg("Nothing to claim for this bet.")]
    NothingToClaim,
}
