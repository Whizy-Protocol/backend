-- ================================================
-- Rollback: Restore original update_market_from_bet trigger
-- ================================================

-- Drop the modified trigger
DROP TRIGGER IF EXISTS trigger_update_market_from_bet ON bets_extended;

-- Note: Rolling back this migration will restore the duplicate update behavior.
-- The original trigger handled INSERT, UPDATE, and DELETE.
-- This will cause double-counting of market stats again.

-- Restore original function (with INSERT handling)
CREATE OR REPLACE FUNCTION update_market_from_bet()
RETURNS TRIGGER AS $$
BEGIN
    -- Handle INSERT
    IF (TG_OP = 'INSERT') THEN
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

-- Recreate trigger with all operations
CREATE TRIGGER trigger_update_market_from_bet
    AFTER INSERT OR UPDATE OR DELETE ON bets_extended
    FOR EACH ROW EXECUTE FUNCTION update_market_from_bet();
