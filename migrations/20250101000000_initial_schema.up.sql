-- ================================================
-- Initial Database Schema Migration
-- ================================================

-- ================================================
-- Helper Functions
-- ================================================

-- Function to update updatedAt timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW."updatedAt" = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ================================================
-- Core Tables
-- ================================================

-- Users table
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    address TEXT NOT NULL UNIQUE,
    email TEXT UNIQUE,
    username TEXT UNIQUE,
    "avatarUrl" TEXT,
    "createdAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_address ON users(address);

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Protocols table
CREATE TABLE protocols (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    "displayName" TEXT,
    "protocolType" INTEGER,
    address TEXT,
    "baseApy" NUMERIC(10,6) NOT NULL DEFAULT 0,
    tvl NUMERIC DEFAULT 0,
    "riskLevel" INTEGER DEFAULT 1,
    "isActive" BOOLEAN NOT NULL DEFAULT true,
    description TEXT,
    "iconUrl" TEXT,
    "createdAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_protocols_address ON protocols(address);

CREATE TRIGGER update_protocols_updated_at
    BEFORE UPDATE ON protocols
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Markets extended table
CREATE TABLE markets_extended (
    id TEXT PRIMARY KEY,
    "blockchainMarketId" BIGINT UNIQUE,
    "marketId" TEXT UNIQUE,
    "adjTicker" TEXT UNIQUE,
    platform TEXT NOT NULL DEFAULT 'base',
    question TEXT,
    description TEXT,
    rules TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    probability INTEGER NOT NULL DEFAULT 50,
    volume NUMERIC(78,18) NOT NULL DEFAULT 0,
    "openInterest" NUMERIC(78,18) NOT NULL DEFAULT 0,
    "endDate" TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    "resolutionDate" TIMESTAMP WITHOUT TIME ZONE,
    result BOOLEAN,
    link TEXT,
    "imageUrl" TEXT,
    "totalPoolSize" NUMERIC(78,18) NOT NULL DEFAULT 0,
    "yesPoolSize" NUMERIC(78,18) NOT NULL DEFAULT 0,
    "noPoolSize" NUMERIC(78,18) NOT NULL DEFAULT 0,
    "countYes" INTEGER NOT NULL DEFAULT 0,
    "countNo" INTEGER NOT NULL DEFAULT 0,
    "currentYield" NUMERIC(78,18) NOT NULL DEFAULT 0,
    "totalYieldEarned" NUMERIC(78,18) NOT NULL DEFAULT 0,
    "createdAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_markets_extended_adjTicker ON markets_extended("adjTicker");
CREATE INDEX idx_markets_extended_blockchainMarketId ON markets_extended("blockchainMarketId") WHERE "blockchainMarketId" IS NOT NULL;
CREATE INDEX idx_markets_extended_marketId ON markets_extended("marketId");
CREATE INDEX idx_markets_extended_status ON markets_extended(status);
CREATE INDEX idx_markets_extended_endDate ON markets_extended("endDate");

CREATE TRIGGER update_markets_extended_updated_at
    BEFORE UPDATE ON markets_extended
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Bets extended table
CREATE TABLE bets_extended (
    id TEXT PRIMARY KEY,
    "blockchainBetId" BIGINT NOT NULL UNIQUE,
    "userId" TEXT NOT NULL REFERENCES users(id),
    "marketId" TEXT,
    position BOOLEAN,
    amount NUMERIC(78,18),
    odds NUMERIC(10,4) NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    payout NUMERIC(78,18),
    "createdAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_bets_extended_blockchainBetId ON bets_extended("blockchainBetId");
CREATE INDEX idx_bets_extended_userId ON bets_extended("userId");
CREATE INDEX idx_bets_extended_marketId ON bets_extended("marketId");
CREATE INDEX idx_bets_extended_status ON bets_extended(status);

CREATE TRIGGER update_bets_extended_updated_at
    BEFORE UPDATE ON bets_extended
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Blockchain events table
CREATE TABLE blockchain_events (
    id TEXT PRIMARY KEY,
    "eventType" TEXT NOT NULL,
    "blockchainId" TEXT NOT NULL,
    "blockNumber" BIGINT NOT NULL,
    "blockTimestamp" BIGINT NOT NULL,
    "transactionHash" TEXT NOT NULL,
    processed BOOLEAN NOT NULL DEFAULT false,
    data TEXT,
    "createdAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE("eventType", "blockchainId")
);

CREATE INDEX idx_blockchain_events_blockNumber ON blockchain_events("blockNumber");
CREATE INDEX idx_blockchain_events_type_processed ON blockchain_events("eventType", processed);

CREATE TRIGGER update_blockchain_events_updated_at
    BEFORE UPDATE ON blockchain_events
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Fee records table
CREATE TABLE fee_records (
    id TEXT PRIMARY KEY,
    "marketId" TEXT,
    "feeType" TEXT NOT NULL,
    amount NUMERIC(78,18) NOT NULL,
    source TEXT NOT NULL,
    "createdAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_fee_records_marketId ON fee_records("marketId");
CREATE INDEX idx_fee_records_feeType ON fee_records("feeType");

-- User yields table
CREATE TABLE user_yields (
    id TEXT PRIMARY KEY,
    "userId" TEXT REFERENCES users(id),
    "marketId" TEXT,
    amount NUMERIC(78,18) NOT NULL,
    "protocolId" TEXT REFERENCES protocols(id),
    "earnedAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_user_yields_userId ON user_yields("userId");
CREATE INDEX idx_user_yields_marketId ON user_yields("marketId");
CREATE INDEX idx_user_yields_protocolId ON user_yields("protocolId");

-- Yield records table
CREATE TABLE yield_records (
    id TEXT PRIMARY KEY,
    "marketId" TEXT NOT NULL,
    "protocolId" TEXT NOT NULL REFERENCES protocols(id),
    amount NUMERIC(78,18) NOT NULL,
    apy NUMERIC(10,6) NOT NULL,
    yield NUMERIC(78,18) NOT NULL,
    period TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    "createdAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_yield_records_marketId ON yield_records("marketId");
CREATE INDEX idx_yield_records_protocolId ON yield_records("protocolId");
CREATE INDEX idx_yield_records_period ON yield_records(period);

-- Sync states table
CREATE TABLE sync_states (
    contract_address TEXT PRIMARY KEY,
    contract_name TEXT NOT NULL,
    last_block BIGINT NOT NULL,
    last_block_hash TEXT
);

-- Sync status table
CREATE TABLE sync_status (
    id TEXT PRIMARY KEY,
    "eventType" TEXT NOT NULL UNIQUE,
    "lastSyncBlock" BIGINT NOT NULL DEFAULT 0,
    "lastSyncTime" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "isActive" BOOLEAN NOT NULL DEFAULT true,
    "createdAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updatedAt" TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_sync_status_updated_at
    BEFORE UPDATE ON sync_status
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ================================================
-- Event Tables (Raw blockchain events)
-- ================================================

-- Auto deposit executed events
CREATE TABLE auto_deposit_executeds (
    id TEXT PRIMARY KEY,
    "user" TEXT NOT NULL,
    protocol TEXT NOT NULL,
    amount NUMERIC NOT NULL,
    success BOOLEAN NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_auto_deposit_executeds_user ON auto_deposit_executeds("user");
CREATE INDEX idx_auto_deposit_executeds_transaction_hash ON auto_deposit_executeds(transaction_hash);

-- Auto withdraw executed events
CREATE TABLE auto_withdraw_executeds (
    id TEXT PRIMARY KEY,
    "user" TEXT NOT NULL,
    protocol TEXT NOT NULL,
    amount NUMERIC NOT NULL,
    success BOOLEAN NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_auto_withdraw_executeds_user ON auto_withdraw_executeds("user");
CREATE INDEX idx_auto_withdraw_executeds_transaction_hash ON auto_withdraw_executeds(transaction_hash);

-- Bet placed events
CREATE TABLE bet_placeds (
    id TEXT PRIMARY KEY,
    bet_id NUMERIC NOT NULL,
    market_id NUMERIC NOT NULL,
    "user" TEXT NOT NULL,
    position BOOLEAN NOT NULL,
    amount NUMERIC NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_bet_placeds_user ON bet_placeds("user");
CREATE INDEX idx_bet_placeds_market_id ON bet_placeds(market_id);
CREATE INDEX idx_bet_placeds_transaction_hash ON bet_placeds(transaction_hash);

-- Market created events
CREATE TABLE market_createds (
    id TEXT PRIMARY KEY,
    market_id NUMERIC NOT NULL,
    question TEXT NOT NULL,
    enddate NUMERIC NOT NULL,
    betting_deadline NUMERIC NOT NULL,
    token_address TEXT NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_market_createds_market_id ON market_createds(market_id);
CREATE INDEX idx_market_createds_transaction_hash ON market_createds(transaction_hash);

-- Market resolved events
CREATE TABLE market_resolveds (
    id TEXT PRIMARY KEY,
    market_id NUMERIC NOT NULL,
    outcome BOOLEAN NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_market_resolveds_market_id ON market_resolveds(market_id);
CREATE INDEX idx_market_resolveds_transaction_hash ON market_resolveds(transaction_hash);

-- Ownership transferred events
CREATE TABLE ownership_transferreds (
    id TEXT PRIMARY KEY,
    previous_owner TEXT NOT NULL,
    new_owner TEXT NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_ownership_transferreds_transaction_hash ON ownership_transferreds(transaction_hash);

-- Paused events
CREATE TABLE pauseds (
    id TEXT PRIMARY KEY,
    account TEXT NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_pauseds_transaction_hash ON pauseds(transaction_hash);

-- Protocol registered events
CREATE TABLE protocol_registereds (
    id TEXT PRIMARY KEY,
    protocol_type BIGINT NOT NULL,
    protocol_address TEXT NOT NULL,
    name TEXT NOT NULL,
    risk_level BIGINT NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_protocol_registereds_protocol_address ON protocol_registereds(protocol_address);
CREATE INDEX idx_protocol_registereds_transaction_hash ON protocol_registereds(transaction_hash);

-- Protocol updated events
CREATE TABLE protocol_updateds (
    id TEXT PRIMARY KEY,
    protocol_address TEXT NOT NULL,
    new_apy NUMERIC NOT NULL,
    new_tvl NUMERIC NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_protocol_updateds_protocol_address ON protocol_updateds(protocol_address);
CREATE INDEX idx_protocol_updateds_transaction_hash ON protocol_updateds(transaction_hash);

-- Unpaused events
CREATE TABLE unpauseds (
    id TEXT PRIMARY KEY,
    account TEXT NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_unpauseds_transaction_hash ON unpauseds(transaction_hash);

-- Winnings claimed events
CREATE TABLE winnings_claimeds (
    id TEXT PRIMARY KEY,
    bet_id NUMERIC NOT NULL,
    "user" TEXT NOT NULL,
    winning_amount NUMERIC NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_winnings_claimeds_user ON winnings_claimeds("user");
CREATE INDEX idx_winnings_claimeds_transaction_hash ON winnings_claimeds(transaction_hash);

-- ================================================
-- Sync Functions for Event Processing
-- ================================================

-- Function: sync_market_created
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

-- Function: sync_market_resolved
CREATE OR REPLACE FUNCTION sync_market_resolved()
RETURNS TRIGGER AS $$
BEGIN
    -- Update market status and result
    UPDATE markets_extended
    SET
        status = 'resolved',
        result = NEW.outcome,
        "resolutionDate" = to_timestamp(NEW.block_timestamp),
        "updatedAt" = NOW()
    WHERE "blockchainMarketId" = NEW.market_id;

    -- Update bets for this market
    -- Mark winning bets as 'won' and losing bets as 'lost'
    UPDATE bets_extended
    SET
        status = CASE
            WHEN position = NEW.outcome THEN 'won'
            ELSE 'lost'
        END,
        "updatedAt" = NOW()
    WHERE "marketId" IN (
        SELECT id FROM markets_extended WHERE "blockchainMarketId" = NEW.market_id
    ) AND status = 'active';

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function: sync_bet_placed
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
        1.0, -- Default odds, can be calculated later
        'active',
        to_timestamp(NEW.block_timestamp),
        NOW()
    )
    ON CONFLICT (id) DO NOTHING;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Function: sync_winnings_claimed
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

-- ================================================
-- Event Triggers
-- ================================================

CREATE TRIGGER sync_market_created_trigger
    AFTER INSERT ON market_createds
    FOR EACH ROW
    EXECUTE FUNCTION sync_market_created();

CREATE TRIGGER sync_market_resolved_trigger
    AFTER INSERT ON market_resolveds
    FOR EACH ROW
    EXECUTE FUNCTION sync_market_resolved();

CREATE TRIGGER sync_bet_placed_trigger
    AFTER INSERT ON bet_placeds
    FOR EACH ROW
    EXECUTE FUNCTION sync_bet_placed();

CREATE TRIGGER sync_winnings_claimed_trigger
    AFTER INSERT ON winnings_claimeds
    FOR EACH ROW
    EXECUTE FUNCTION sync_winnings_claimed();
