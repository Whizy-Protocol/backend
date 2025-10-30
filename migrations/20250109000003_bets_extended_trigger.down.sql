-- Rollback: Remove trigger for auto-updating markets from bets_extended

DROP TRIGGER IF EXISTS trigger_update_market_from_bet ON bets_extended;
DROP FUNCTION IF EXISTS update_market_from_bet();
