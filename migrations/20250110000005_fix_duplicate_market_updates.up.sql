-- ================================================
-- Migration: Fix duplicate market stat updates
-- The sync_bet_placed trigger already updates market stats,
-- so we need to prevent update_market_from_bet from doing it again on INSERT
-- ================================================

-- Drop the existing trigger
DROP TRIGGER IF EXISTS trigger_update_market_from_bet ON bets_extended;

-- Recreate the function to only handle UPDATE and DELETE (not INSERT)
CREATE OR REPLACE FUNCTION update_market_from_bet()
RETURNS TRIGGER AS $$
BEGIN
    -- Handle UPDATE
    IF (TG_OP = 'UPDATE') THEN
        -- Remove old bet amounts
        IF OLD.position = TRUE THEN
            UPDATE markets_extended
            SET
                "yesPoolSize" = "yesPoolSize" - OLD.amount,
                "totalPoolSize" = "totalPoolSize" - OLD.amount,
                "countYes" = "countYes" - 1,
                "updatedAt" = NOW()
            WHERE id = OLD."marketId";
        ELSE
            UPDATE markets_extended
            SET
                "noPoolSize" = "noPoolSize" - OLD.amount,
                "totalPoolSize" = "totalPoolSize" - OLD.amount,
                "countNo" = "countNo" - 1,
                "updatedAt" = NOW()
            WHERE id = OLD."marketId";
        END IF;

        -- Add new bet amounts
        IF NEW.position = TRUE THEN
            UPDATE markets_extended
            SET
                "yesPoolSize" = "yesPoolSize" + NEW.amount,
                "totalPoolSize" = "totalPoolSize" + NEW.amount,
                "countYes" = "countYes" + 1,
                "updatedAt" = NOW()
            WHERE id = NEW."marketId";
        ELSE
            UPDATE markets_extended
            SET
                "noPoolSize" = "noPoolSize" + NEW.amount,
                "totalPoolSize" = "totalPoolSize" + NEW.amount,
                "countNo" = "countNo" + 1,
                "updatedAt" = NOW()
            WHERE id = NEW."marketId";
        END IF;
        RETURN NEW;
    END IF;

    -- Handle DELETE
    IF (TG_OP = 'DELETE') THEN
        IF OLD.position = TRUE THEN
            UPDATE markets_extended
            SET
                "yesPoolSize" = "yesPoolSize" - OLD.amount,
                "totalPoolSize" = "totalPoolSize" - OLD.amount,
                "countYes" = "countYes" - 1,
                "updatedAt" = NOW()
            WHERE id = OLD."marketId";
        ELSE
            UPDATE markets_extended
            SET
                "noPoolSize" = "noPoolSize" - OLD.amount,
                "totalPoolSize" = "totalPoolSize" - OLD.amount,
                "countNo" = "countNo" - 1,
                "updatedAt" = NOW()
            WHERE id = OLD."marketId";
        END IF;
        RETURN OLD;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Recreate the trigger to only fire on UPDATE and DELETE (not INSERT)
CREATE TRIGGER trigger_update_market_from_bet
    AFTER UPDATE OR DELETE ON bets_extended
    FOR EACH ROW EXECUTE FUNCTION update_market_from_bet();

-- Fix the existing market stats by recalculating from actual bets
UPDATE markets_extended me
SET 
    "yesPoolSize" = COALESCE((
        SELECT SUM(amount) FROM bets_extended 
        WHERE "marketId" = me.id AND position = true
    ), 0),
    "noPoolSize" = COALESCE((
        SELECT SUM(amount) FROM bets_extended 
        WHERE "marketId" = me.id AND position = false
    ), 0),
    "totalPoolSize" = COALESCE((
        SELECT SUM(amount) FROM bets_extended 
        WHERE "marketId" = me.id
    ), 0),
    "countYes" = COALESCE((
        SELECT COUNT(*) FROM bets_extended 
        WHERE "marketId" = me.id AND position = true
    ), 0),
    "countNo" = COALESCE((
        SELECT COUNT(*) FROM bets_extended 
        WHERE "marketId" = me.id AND position = false
    ), 0),
    volume = COALESCE((
        SELECT SUM(amount) FROM bets_extended 
        WHERE "marketId" = me.id
    ), 0);

-- Comment for documentation
COMMENT ON TRIGGER trigger_update_market_from_bet ON bets_extended IS 'Updates market stats when bets are modified or deleted (INSERT is handled by sync_bet_placed)';
