-- Down migration: Reset odds to 1.0 (not recommended, but provided for completeness)
-- This would revert the odds calculation if needed

UPDATE bets_extended
SET odds = '1.0',
    "updatedAt" = CURRENT_TIMESTAMP
WHERE odds != '1.0';
