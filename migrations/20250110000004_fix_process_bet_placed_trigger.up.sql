-- ================================================
-- Migration: Fix process_bet_placed trigger to remove bet_id references
-- This trigger was created before and still references the old bet_id column
-- ================================================

-- Drop the old trigger first
DROP TRIGGER IF EXISTS trigger_process_bet_placed ON bet_placeds;

-- Drop the old function
DROP FUNCTION IF EXISTS process_bet_placed();

-- Note: The sync_bet_placed() function already handles everything we need,
-- so we don't need to recreate this trigger. The sync_bet_placed_trigger 
-- is sufficient and already updated in migration 20250110000003.

-- Comment for documentation
COMMENT ON TRIGGER sync_bet_placed_trigger ON bet_placeds IS 'Syncs bet_placeds to bets_extended and updates market stats (post-vault update)';
