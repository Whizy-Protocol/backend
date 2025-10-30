-- Add enddate column to market_createds table
ALTER TABLE market_createds ADD COLUMN IF NOT EXISTS enddate NUMERIC;

-- Update existing rows to have a default value
UPDATE market_createds SET enddate = 0 WHERE enddate IS NULL;

-- Make the column NOT NULL after populating it
ALTER TABLE market_createds ALTER COLUMN enddate SET NOT NULL;

-- Update the sync_market_created function to handle enddate
CREATE OR REPLACE FUNCTION sync_market_created()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO markets_extended (
        id,
        "blockchainMarketId",
        question,
        "endDate",
        status,
        "createdAt",
        "updatedAt"
    ) VALUES (
        gen_random_uuid()::text,
        NEW.market_id,
        NEW.question,
        CASE 
            WHEN NEW.enddate IS NOT NULL AND NEW.enddate > 0 
            THEN to_timestamp(NEW.enddate)
            ELSE to_timestamp(NEW.betting_deadline)
        END,
        'active',
        to_timestamp(NEW.block_timestamp),
        NOW()
    )
    ON CONFLICT ("blockchainMarketId") DO NOTHING;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
