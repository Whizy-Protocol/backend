-- Migration: Add MarketVaultRebalanced event table
-- Description: Creates table for tracking market vault rebalancing events
-- Date: 2025-01-31

-- MarketVaultRebalanced events
CREATE TABLE IF NOT EXISTS market_vault_rebalanceds (
    id TEXT PRIMARY KEY,
    market_id NUMERIC NOT NULL,
    amount NUMERIC NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_market_vault_rebalanceds_market_id ON market_vault_rebalanceds(market_id);
CREATE INDEX idx_market_vault_rebalanceds_tx_hash ON market_vault_rebalanceds(transaction_hash);
CREATE INDEX idx_market_vault_rebalanceds_block_number ON market_vault_rebalanceds(block_number);
CREATE INDEX idx_market_vault_rebalanceds_block_timestamp ON market_vault_rebalanceds(block_timestamp);

-- Create view for market rebalancing history
CREATE VIEW market_rebalancing_history AS
SELECT 
    m.market_id,
    mc.question as market_question,
    COUNT(m.id) as rebalance_count,
    SUM(m.amount) as total_rebalanced_amount,
    MIN(m.block_timestamp) as first_rebalance_time,
    MAX(m.block_timestamp) as last_rebalance_time,
    AVG(m.amount) as avg_rebalance_amount
FROM market_vault_rebalanceds m
LEFT JOIN market_createds mc ON m.market_id = mc.market_id
GROUP BY m.market_id, mc.question;

-- Comments for documentation
COMMENT ON TABLE market_vault_rebalanceds IS 'Tracks when market vaults are rebalanced to optimal yield protocols';
COMMENT ON VIEW market_rebalancing_history IS 'Provides aggregated statistics on market vault rebalancing activity';
