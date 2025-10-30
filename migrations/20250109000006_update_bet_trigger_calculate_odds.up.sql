-- Migration: Update bet_placed trigger to automatically calculate odds from market pool sizes
-- This fixes the hardcoded 1.0 odds in the original trigger

CREATE OR REPLACE FUNCTION process_bet_placed()
RETURNS TRIGGER AS $$
DECLARE
    v_market_uuid TEXT;
    v_bet_uuid TEXT;
    v_user_uuid TEXT;
    v_yes_pool NUMERIC;
    v_no_pool NUMERIC;
    v_total_pool NUMERIC;
    v_calculated_odds NUMERIC;
BEGIN
    -- Find the market UUID and pool sizes from blockchain market ID
    SELECT 
        id,
        "yesPoolSize",
        "noPoolSize",
        "totalPoolSize"
    INTO 
        v_market_uuid,
        v_yes_pool,
        v_no_pool,
        v_total_pool
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

    -- Calculate odds based on position and pool sizes
    -- For YES: odds = totalPoolSize / yesPoolSize
    -- For NO: odds = totalPoolSize / noPoolSize
    IF NEW.position = true THEN
        -- YES position
        IF v_yes_pool > 0 THEN
            v_calculated_odds := ROUND((v_total_pool / v_yes_pool)::numeric, 2);
        ELSE
            v_calculated_odds := 1.0;
        END IF;
    ELSE
        -- NO position
        IF v_no_pool > 0 THEN
            v_calculated_odds := ROUND((v_total_pool / v_no_pool)::numeric, 2);
        ELSE
            v_calculated_odds := 1.0;
        END IF;
    END IF;

    -- Generate UUID for the bet
    v_bet_uuid := gen_random_uuid()::text;

    -- Insert bet into bets_extended with calculated odds
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
        v_calculated_odds,  -- Use calculated odds instead of hardcoded 1.0
        'active',
        NOW(),
        NOW()
    );

    RAISE NOTICE 'Successfully processed bet % for market % with odds % (market update will be handled by bets_extended trigger)', 
        NEW.bet_id, v_market_uuid, v_calculated_odds;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- The trigger already exists from the previous migration, just updating the function
COMMENT ON FUNCTION process_bet_placed() IS 
'Automatically processes bet_placed events: creates bet in bets_extended with calculated odds based on market pool sizes and updates market pools';
