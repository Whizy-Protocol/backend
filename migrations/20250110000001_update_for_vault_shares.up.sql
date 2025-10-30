-- ================================================
-- Migration: Update schema for new vault-based contract
-- New contract uses ERC4626 vaults and shares instead of direct amounts
-- ================================================

-- Add vault address to markets_extended
ALTER TABLE markets_extended 
ADD COLUMN IF NOT EXISTS "vaultAddress" TEXT;

-- Add shares tracking to markets_extended (replacing totalYesAmount/totalNoAmount conceptually)
ALTER TABLE markets_extended
ADD COLUMN IF NOT EXISTS "totalYesShares" NUMERIC(78,18) NOT NULL DEFAULT 0,
ADD COLUMN IF NOT EXISTS "totalNoShares" NUMERIC(78,18) NOT NULL DEFAULT 0;

-- Add yield withdrawn tracking (yield that has been withdrawn from protocols)
ALTER TABLE markets_extended
ADD COLUMN IF NOT EXISTS "yieldWithdrawn" NUMERIC(78,18) NOT NULL DEFAULT 0;

-- Update bet_placeds table to include shares emitted in event
ALTER TABLE bet_placeds
ADD COLUMN IF NOT EXISTS shares NUMERIC;

-- Create index on vault address
CREATE INDEX IF NOT EXISTS idx_markets_extended_vaultAddress 
ON markets_extended("vaultAddress") WHERE "vaultAddress" IS NOT NULL;

-- Make blockchainBetId nullable in bets_extended since new contract uses position-based tracking
ALTER TABLE bets_extended 
ALTER COLUMN "blockchainBetId" DROP NOT NULL;

-- Add shares column to bets_extended for tracking user's shares
ALTER TABLE bets_extended
ADD COLUMN IF NOT EXISTS shares NUMERIC(78,18);

-- Update market_createds table to include vault address from event
ALTER TABLE market_createds
ADD COLUMN IF NOT EXISTS vault_address TEXT;

-- ================================================
-- Update sync functions for new contract structure
-- NOTE: These functions are superseded by migration 20250110000003
-- which fixes issues with field references (bet_id, betting_deadline, etc.)
-- ================================================

-- Update sync_market_created to handle vault address
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
        CASE 
            WHEN NEW.enddate IS NOT NULL AND NEW.enddate > 0 
            THEN to_timestamp(NEW.enddate)
            ELSE to_timestamp(NEW.betting_deadline)
        END,
        NEW.vault_address,
        'active',
        to_timestamp(NEW.block_timestamp),
        NOW()
    )
    ON CONFLICT ("blockchainMarketId") DO UPDATE SET
        "vaultAddress" = EXCLUDED."vaultAddress",
        "updatedAt" = NOW();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Update sync_bet_placed to handle shares
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

    -- Insert into bets_extended with shares
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
        NEW.bet_id,
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

    -- Update market shares
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

-- Update sync_winnings_claimed for new structure (no bet_id, uses marketId + user)
CREATE OR REPLACE FUNCTION sync_winnings_claimed()
RETURNS TRIGGER AS $$
DECLARE
    bet_records RECORD;
BEGIN
    -- New contract doesn't track individual bet IDs
    -- Find bets by user address and mark them as claimed
    FOR bet_records IN
        SELECT be.id
        FROM bets_extended be
        JOIN users u ON be."userId" = u.id
        JOIN markets_extended me ON be."marketId" = me.id
        WHERE u.address = NEW.user
        AND me."blockchainMarketId" = (
            SELECT market_id FROM winnings_claimeds WHERE id = NEW.id LIMIT 1
        )
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
COMMENT ON COLUMN markets_extended."vaultAddress" IS 'Address of the MarketVault (ERC4626) for this market';
COMMENT ON COLUMN markets_extended."totalYesShares" IS 'Total YES shares issued by the vault';
COMMENT ON COLUMN markets_extended."totalNoShares" IS 'Total NO shares issued by the vault';
COMMENT ON COLUMN markets_extended."yieldWithdrawn" IS 'Total yield withdrawn from yield protocols';
COMMENT ON COLUMN bets_extended.shares IS 'Number of vault shares representing this bet position';
COMMENT ON COLUMN bet_placeds.shares IS 'Shares emitted by vault for this bet';
