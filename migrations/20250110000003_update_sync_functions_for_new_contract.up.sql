-- ================================================
-- Migration: Update sync functions for new contract structure
-- Fixes the sync functions to work with the updated event schema
-- ================================================

-- Update sync_market_created to handle new schema (end_time instead of betting_deadline)
CREATE OR REPLACE FUNCTION sync_market_created()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO markets_extended (
        id,
        "blockchainMarketId",
        question,
        "endDate",
        "vaultAddress",
        status,
        "createdAt",
        "updatedAt"
    ) VALUES (
        gen_random_uuid()::text,
        NEW.market_id,
        NEW.question,
        to_timestamp(NEW.end_time),
        NEW.vault_address,
        'active',
        to_timestamp(NEW.block_timestamp),
        NOW()
    )
    ON CONFLICT ("blockchainMarketId") DO UPDATE SET
        "vaultAddress" = EXCLUDED."vaultAddress",
        "endDate" = EXCLUDED."endDate",
        "updatedAt" = NOW();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Update sync_bet_placed to work without bet_id (no longer exists in new contract)
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

    -- Insert into bets_extended with shares (no bet_id needed anymore)
    -- Use a combination of user, market, and block as unique identifier
    INSERT INTO bets_extended (
        id,
        "blockchainBetId",
        "userId",
        "marketId",
        position,
        amount,
        shares,
        odds,
        status,
        "createdAt",
        "updatedAt"
    ) VALUES (
        NEW.id,
        NULL, -- No bet_id in new contract
        user_id_var,
        market_id_var,
        NEW.position,
        NEW.amount,
        NEW.shares,
        1.0, -- Default odds, will be calculated by trigger
        'active',
        to_timestamp(NEW.block_timestamp),
        NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    -- Update market shares and pool sizes
    IF NEW.position THEN
        UPDATE markets_extended
        SET "totalYesShares" = "totalYesShares" + COALESCE(NEW.shares, 0),
            "yesPoolSize" = "yesPoolSize" + NEW.amount,
            "totalPoolSize" = "totalPoolSize" + NEW.amount,
            "countYes" = "countYes" + 1,
            volume = volume + NEW.amount,
            "updatedAt" = NOW()
        WHERE "blockchainMarketId" = NEW.market_id;
    ELSE
        UPDATE markets_extended
        SET "totalNoShares" = "totalNoShares" + COALESCE(NEW.shares, 0),
            "noPoolSize" = "noPoolSize" + NEW.amount,
            "totalPoolSize" = "totalPoolSize" + NEW.amount,
            "countNo" = "countNo" + 1,
            volume = volume + NEW.amount,
            "updatedAt" = NOW()
        WHERE "blockchainMarketId" = NEW.market_id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Update sync_winnings_claimed for new structure (uses market_id instead of bet_id)
CREATE OR REPLACE FUNCTION sync_winnings_claimed()
RETURNS TRIGGER AS $$
DECLARE
    bet_records RECORD;
BEGIN
    -- New contract uses market_id instead of bet_id
    -- Find all bets by user address for this market and mark them as claimed
    FOR bet_records IN
        SELECT be.id
        FROM bets_extended be
        JOIN users u ON be."userId" = u.id
        JOIN markets_extended me ON be."marketId" = me.id
        WHERE u.address = NEW.user
        AND me."blockchainMarketId" = NEW.market_id
        AND be.status IN ('active', 'won')
    LOOP
        UPDATE bets_extended
        SET
            payout = NEW.winning_amount,
            status = 'claimed',
            "updatedAt" = NOW()
        WHERE id = bet_records.id;
    END LOOP;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Comment for documentation
COMMENT ON FUNCTION sync_market_created() IS 'Syncs MarketCreated events to markets_extended table (post-vault update)';
COMMENT ON FUNCTION sync_bet_placed() IS 'Syncs BetPlaced events to bets_extended table (post-vault update, no bet_id)';
COMMENT ON FUNCTION sync_winnings_claimed() IS 'Syncs WinningsClaimed events to bets_extended table (post-vault update, uses market_id)';
