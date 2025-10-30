-- ================================================
-- Rollback: Restore old sync functions
-- ================================================

-- Restore original sync_market_created function
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

-- Restore original sync_bet_placed function
CREATE OR REPLACE FUNCTION sync_bet_placed()
RETURNS TRIGGER AS $$
DECLARE
    user_id_var text;
    market_id_var text;
BEGIN
    -- Get user_id from users table by address
    SELECT id INTO user_id_var FROM users WHERE address = NEW.user LIMIT 1;

    -- If user doesn't exist, create it
    IF user_id_var IS NULL THEN
        INSERT INTO users (id, address, "createdAt", "updatedAt")
        VALUES (gen_random_uuid()::text, NEW.user, NOW(), NOW())
        RETURNING id INTO user_id_var;
    END IF;

    -- Get market_id from markets_extended by blockchainMarketId
    SELECT id INTO market_id_var FROM markets_extended WHERE "blockchainMarketId" = NEW.market_id LIMIT 1;

    -- Insert into bets_extended
    INSERT INTO bets_extended (
        id,
        "blockchainBetId",
        "userId",
        "marketId",
        position,
        amount,
        odds,
        status,
        "createdAt",
        "updatedAt"
    ) VALUES (
        NEW.id,
        NEW.bet_id,
        user_id_var,
        market_id_var,
        NEW.position,
        NEW.amount,
        1.0,
        'active',
        to_timestamp(NEW.block_timestamp),
        NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Restore original sync_winnings_claimed function
CREATE OR REPLACE FUNCTION sync_winnings_claimed()
RETURNS TRIGGER AS $$
DECLARE
    bet_record RECORD;
BEGIN
    -- Find the corresponding bet by blockchainBetId
    SELECT * INTO bet_record
    FROM bets_extended
    WHERE "blockchainBetId" = NEW.bet_id
    LIMIT 1;

    -- If bet found, update it with payout and status
    IF bet_record.id IS NOT NULL THEN
        UPDATE bets_extended
        SET
            payout = NEW.winning_amount,
            status = 'claimed',
            "updatedAt" = NOW()
        WHERE id = bet_record.id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
