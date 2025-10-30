-- Rollback migration for volume calculation fix

-- Drop the trigger
DROP TRIGGER IF EXISTS trigger_update_market_volume ON markets_extended;

-- Drop the function
DROP FUNCTION IF EXISTS update_market_volume();

-- Remove comments
COMMENT ON COLUMN markets_extended.volume IS NULL;
COMMENT ON COLUMN markets_extended."totalPoolSize" IS NULL;
