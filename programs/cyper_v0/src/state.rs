use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct CyperMarket {
    pub bump: u8,
    pub authority: Pubkey,              // admin wallet — THE top-level authority
    pub treasury: Pubkey,               // fee collection wallet
    pub default_protocol_fee_bps: u16,  // e.g. 50 = 0.50%
    pub market_count: u64,
}

#[derive(InitSpace)]
#[account]
pub struct Market {
    pub bump: u8,
    pub creator: Pubkey,                // creator wallet — can lock, settle, withdraw bond+fees
    pub market_index: u64,
    pub market_type: MarketType,        // YesNo | MultiOutcome | Accuracy
    pub category: MarketCategory,       // Crypto | Politics | Sports | Tech | Economy | Culture | Beyond
    pub status: MarketStatus,           // Open | Locked | Settled | Voided
    #[max_len(200)]
    pub question: String,               // max 200 bytes
    pub creator_bond: u64,
    pub lp_fee_bps: u16,
    pub protocol_fee_bps: u16,
    pub resolution_deadline: i64,
    pub created_at: i64,
    pub total_bets: u64,
    pub vault: Pubkey,
    pub token_mint: Pubkey,
    pub market_group: Option<Pubkey>,   // None for standalone, Some for tiered
    pub market_data: MarketData,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum MarketType {
    YesNo,
    MultiOutcome,
    Accuracy,
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum MarketData {
    // Placeholder - you can define specific data structures for each market type here
    None,
}
