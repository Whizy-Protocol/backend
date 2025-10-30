-- Rollback: Remove bet_placed trigger and function

DROP TRIGGER IF EXISTS trigger_process_bet_placed ON bet_placeds;
DROP FUNCTION IF EXISTS process_bet_placed();
