-- ================================================
-- Rollback: Cannot restore old process_bet_placed function
-- The old function referenced bet_id which no longer exists
-- ================================================

-- This migration cannot be rolled back because the old function
-- referenced the bet_id column which has been removed from bet_placeds table.
-- If you need to rollback, you must first rollback migration 20250110000002
-- which removes the bet_id column.

-- No action needed for rollback
