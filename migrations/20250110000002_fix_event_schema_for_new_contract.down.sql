-- ================================================
-- Rollback: Restore old event schema
-- ================================================

-- Restore bet_id to bet_placeds
ALTER TABLE bet_placeds
ADD COLUMN IF NOT EXISTS bet_id NUMERIC;

-- Restore betting_deadline and rename end_time back to enddate
ALTER TABLE market_createds
RENAME COLUMN end_time TO enddate;

ALTER TABLE market_createds
ADD COLUMN IF NOT EXISTS betting_deadline NUMERIC NOT NULL DEFAULT 0;

-- Restore bet_id to winnings_claimeds
ALTER TABLE winnings_claimeds
RENAME COLUMN market_id TO bet_id;

-- Update index names
DROP INDEX IF EXISTS idx_winnings_claimeds_market_id;
CREATE INDEX IF NOT EXISTS idx_winnings_claimeds_bet_id ON winnings_claimeds(bet_id);
