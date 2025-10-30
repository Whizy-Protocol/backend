-- Down migration: Revert to the original trigger with hardcoded 1.0 odds
-- This restores the original behavior from 20250109000002_bet_placed_triggers.up.sql

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
