-- Revert sync_market_created function to original
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
        to_timestamp(NEW.betting_deadline),
        'active',
        to_timestamp(NEW.block_timestamp),
        NOW()
    )
    ON CONFLICT ("blockchainMarketId") DO NOTHING;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Remove enddate column from market_createds table
ALTER TABLE market_createds DROP COLUMN IF EXISTS enddate;
