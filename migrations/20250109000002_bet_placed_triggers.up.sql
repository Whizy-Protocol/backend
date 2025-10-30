-- Migration: Auto-sync bets from bet_placeds to bets_extended and update markets
-- This trigger runs whenever a new bet is indexed into bet_placeds

-- Function to process bet_placed events and update markets
CREATE OR REPLACE FUNCTION process_bet_placed()
RETURNS TRIGGER AS $$
DECLARE
    v_market_uuid TEXT;
    v_bet_uuid TEXT;
    v_user_uuid TEXT;
BEGIN
    -- Find the market UUID from blockchain market ID
    SELECT id INTO v_market_uuid
    FROM markets_extended
    WHERE "blockchainMarketId" = NEW.market_id;

    -- If market doesn't exist, skip processing
    IF v_market_uuid IS NULL THEN
        RAISE NOTICE 'Market with blockchainMarketId % not found, skipping bet %', NEW.market_id, NEW.bet_id;
        RETURN NEW;
    END IF;

    -- Check if bet already exists in bets_extended
    IF EXISTS (SELECT 1 FROM bets_extended WHERE "blockchainBetId" = NEW.bet_id) THEN
        RAISE NOTICE 'Bet % already exists in bets_extended, skipping', NEW.bet_id;
        RETURN NEW;
    END IF;

    -- Get or create user by address
    SELECT id INTO v_user_uuid
    FROM users
    WHERE address = NEW."user";

    -- If user doesn't exist, create one
    IF v_user_uuid IS NULL THEN
        v_user_uuid := gen_random_uuid()::text;
        INSERT INTO users (id, address, "createdAt", "updatedAt")
        VALUES (v_user_uuid, NEW."user", NOW(), NOW());
        RAISE NOTICE 'Created new user % for address %', v_user_uuid, NEW."user";
    END IF;

    -- Generate UUID for the bet
    v_bet_uuid := gen_random_uuid()::text;

    -- Insert bet into bets_extended
    INSERT INTO bets_extended (
        id,
        "userId",
        "marketId",
        "blockchainBetId",
        position,
        amount,
        odds,
        status,
        "createdAt",
        "updatedAt"
    ) VALUES (
        v_bet_uuid,
        v_user_uuid,
        v_market_uuid,
        NEW.bet_id,
        NEW.position,
        NEW.amount,
        1.0,
        'active',
        NOW(),
        NOW()
    );

    RAISE NOTICE 'Successfully processed bet % for market % (market update will be handled by bets_extended trigger)', NEW.bet_id, v_market_uuid;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger on bet_placeds table
DROP TRIGGER IF EXISTS trigger_process_bet_placed ON bet_placeds;

CREATE TRIGGER trigger_process_bet_placed
    AFTER INSERT ON bet_placeds
    FOR EACH ROW
    EXECUTE FUNCTION process_bet_placed();

-- Add comment to explain the trigger
COMMENT ON TRIGGER trigger_process_bet_placed ON bet_placeds IS 
'Automatically processes bet_placed events: creates bet in bets_extended and updates market pools';
