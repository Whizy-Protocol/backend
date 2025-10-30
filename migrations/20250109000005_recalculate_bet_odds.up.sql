-- Migration: Recalculate odds for existing bets based on market pool sizes
-- This fixes bets that were synced with hardcoded 1.0 odds

-- Recalculate odds based on the formula:
-- For YES position: odds = totalPoolSize / yesPoolSize
-- For NO position: odds = totalPoolSize / noPoolSize

UPDATE bets_extended b
SET odds = CASE 
    WHEN b.position = true THEN 
        CASE 
            WHEN m."yesPoolSize" > 0 THEN 
                ROUND((m."totalPoolSize" / m."yesPoolSize")::numeric, 2)
            ELSE 1.0
        END
    WHEN b.position = false THEN 
        CASE 
            WHEN m."noPoolSize" > 0 THEN 
                ROUND((m."totalPoolSize" / m."noPoolSize")::numeric, 2)
            ELSE 1.0
        END
    ELSE 1.0
END,
"updatedAt" = CURRENT_TIMESTAMP
FROM markets_extended m
WHERE b."marketId" = m.id
AND (b.odds = '1.0' OR b.odds = '1.00');
