-- Migration: Auto-update markets when bets_extended is inserted/updated/deleted
-- This ensures market pools are always in sync with bets

-- Function to update market when bet changes
CREATE OR REPLACE FUNCTION update_market_from_bet()
RETURNS TRIGGER AS $$
BEGIN
    -- Handle INSERT
    IF (TG_OP = 'INSERT') THEN
        IF NEW.position = TRUE THEN
            -- YES bet
            UPDATE markets_extended
            SET 
                "yesPoolSize" = "yesPoolSize" + NEW.amount,
                "totalPoolSize" = "totalPoolSize" + NEW.amount,
                "countYes" = "countYes" + 1,
                "updatedAt" = NOW()
            WHERE id = NEW."marketId";
        ELSE
            -- NO bet
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

    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Drop existing trigger if it exists
DROP TRIGGER IF EXISTS trigger_update_market_from_bet ON bets_extended;

-- Create trigger on bets_extended
CREATE TRIGGER trigger_update_market_from_bet
    AFTER INSERT OR UPDATE OR DELETE ON bets_extended
    FOR EACH ROW
    EXECUTE FUNCTION update_market_from_bet();

-- Add comment
COMMENT ON TRIGGER trigger_update_market_from_bet ON bets_extended IS 
'Automatically updates market pools and counts whenever a bet is inserted, updated, or deleted';
