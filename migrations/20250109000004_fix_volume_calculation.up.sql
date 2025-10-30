-- Migration to fix volume calculation
-- Volume should always equal yesPoolSize + noPoolSize

-- Step 1: Update all existing records to have correct volume
UPDATE markets_extended
SET volume = "yesPoolSize" + "noPoolSize",
    "updatedAt" = CURRENT_TIMESTAMP
WHERE volume != "yesPoolSize" + "noPoolSize";

-- Step 2: Create a trigger to automatically update volume when pools change
CREATE OR REPLACE FUNCTION update_market_volume()
RETURNS TRIGGER AS $$
BEGIN
    -- Automatically calculate volume as sum of yes and no pool sizes
    NEW.volume = NEW."yesPoolSize" + NEW."noPoolSize";
    NEW."totalPoolSize" = NEW."yesPoolSize" + NEW."noPoolSize";
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Step 3: Attach trigger to markets_extended table
DROP TRIGGER IF EXISTS trigger_update_market_volume ON markets_extended;

CREATE TRIGGER trigger_update_market_volume
    BEFORE INSERT OR UPDATE OF "yesPoolSize", "noPoolSize" ON markets_extended
    FOR EACH ROW
    EXECUTE FUNCTION update_market_volume();

-- Step 4: Add a comment to document this behavior
COMMENT ON COLUMN markets_extended.volume IS 'Total trading volume calculated as yesPoolSize + noPoolSize. Automatically updated by trigger.';
COMMENT ON COLUMN markets_extended."totalPoolSize" IS 'Total pool size calculated as yesPoolSize + noPoolSize. Automatically updated by trigger.';
