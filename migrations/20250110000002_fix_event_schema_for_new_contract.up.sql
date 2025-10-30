-- ================================================
-- Migration: Fix event schema for updated contract
-- This migration aligns raw event tables with the new contract structure
-- ================================================

-- Fix bet_placeds table: Remove bet_id column (no longer in contract events)
-- The new contract event is: BetPlaced(uint256 indexed marketId, address indexed user, bool position, uint256 amount, uint256 shares)
ALTER TABLE bet_placeds
DROP COLUMN IF EXISTS bet_id;

-- Fix market_createds table: Rename columns to match new contract
-- The new contract event is: MarketCreated(uint256 indexed marketId, string question, uint256 endTime, address token, address vault)
-- Remove betting_deadline (not in new contract)
ALTER TABLE market_createds
DROP COLUMN IF EXISTS betting_deadline;

-- Rename enddate to end_time for consistency
ALTER TABLE market_createds
RENAME COLUMN enddate TO end_time;

-- Fix winnings_claimeds table: Change bet_id to market_id
-- The new contract event is: WinningsClaimed(uint256 indexed marketId, address indexed user, uint256 amount)
ALTER TABLE winnings_claimeds
RENAME COLUMN bet_id TO market_id;

-- Update index names to reflect column changes
DROP INDEX IF EXISTS idx_winnings_claimeds_bet_id;
CREATE INDEX IF NOT EXISTS idx_winnings_claimeds_market_id ON winnings_claimeds(market_id);

-- Add comment for documentation
COMMENT ON TABLE bet_placeds IS 'Raw BetPlaced events from WhizyPredictionMarket contract (post-vault update)';
COMMENT ON TABLE market_createds IS 'Raw MarketCreated events from WhizyPredictionMarket contract (post-vault update)';
COMMENT ON TABLE winnings_claimeds IS 'Raw WinningsClaimed events from WhizyPredictionMarket contract (post-vault update)';
