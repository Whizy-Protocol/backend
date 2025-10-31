-- Migration: Add RebalancerDelegation event tables
-- Description: Creates tables for tracking RebalancerDelegation contract events
-- Date: 2025-01-31

-- AutoRebalanceEnabled events
CREATE TABLE IF NOT EXISTS auto_rebalance_enableds (
    id TEXT PRIMARY KEY,
    "user" TEXT NOT NULL,
    risk_profile INTEGER NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_auto_rebalance_enableds_user ON auto_rebalance_enableds("user");
CREATE INDEX idx_auto_rebalance_enableds_tx_hash ON auto_rebalance_enableds(transaction_hash);
CREATE INDEX idx_auto_rebalance_enableds_block_number ON auto_rebalance_enableds(block_number);

-- AutoRebalanceDisabled events
CREATE TABLE IF NOT EXISTS auto_rebalance_disableds (
    id TEXT PRIMARY KEY,
    "user" TEXT NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_auto_rebalance_disableds_user ON auto_rebalance_disableds("user");
CREATE INDEX idx_auto_rebalance_disableds_tx_hash ON auto_rebalance_disableds(transaction_hash);
CREATE INDEX idx_auto_rebalance_disableds_block_number ON auto_rebalance_disableds(block_number);

-- Deposited events
CREATE TABLE IF NOT EXISTS depositeds (
    id TEXT PRIMARY KEY,
    "user" TEXT NOT NULL,
    amount NUMERIC NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_depositeds_user ON depositeds("user");
CREATE INDEX idx_depositeds_tx_hash ON depositeds(transaction_hash);
CREATE INDEX idx_depositeds_block_number ON depositeds(block_number);

-- Withdrawn events
CREATE TABLE IF NOT EXISTS withdrawns (
    id TEXT PRIMARY KEY,
    "user" TEXT NOT NULL,
    amount NUMERIC NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_withdrawns_user ON withdrawns("user");
CREATE INDEX idx_withdrawns_tx_hash ON withdrawns(transaction_hash);
CREATE INDEX idx_withdrawns_block_number ON withdrawns(block_number);

-- Rebalanced events
CREATE TABLE IF NOT EXISTS rebalanceds (
    id TEXT PRIMARY KEY,
    "user" TEXT NOT NULL,
    operator TEXT NOT NULL,
    amount NUMERIC NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_rebalanceds_user ON rebalanceds("user");
CREATE INDEX idx_rebalanceds_operator ON rebalanceds(operator);
CREATE INDEX idx_rebalanceds_tx_hash ON rebalanceds(transaction_hash);
CREATE INDEX idx_rebalanceds_block_number ON rebalanceds(block_number);

-- OperatorAdded events
CREATE TABLE IF NOT EXISTS operator_addeds (
    id TEXT PRIMARY KEY,
    operator TEXT NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_operator_addeds_operator ON operator_addeds(operator);
CREATE INDEX idx_operator_addeds_tx_hash ON operator_addeds(transaction_hash);
CREATE INDEX idx_operator_addeds_block_number ON operator_addeds(block_number);

-- OperatorRemoved events
CREATE TABLE IF NOT EXISTS operator_removeds (
    id TEXT PRIMARY KEY,
    operator TEXT NOT NULL,
    block_number NUMERIC NOT NULL,
    block_timestamp NUMERIC NOT NULL,
    transaction_hash TEXT NOT NULL
);

CREATE INDEX idx_operator_removeds_operator ON operator_removeds(operator);
CREATE INDEX idx_operator_removeds_tx_hash ON operator_removeds(transaction_hash);
CREATE INDEX idx_operator_removeds_block_number ON operator_removeds(block_number);

-- Create materialized view for active auto-rebalance users
CREATE MATERIALIZED VIEW IF NOT EXISTS active_auto_rebalance_users AS
SELECT DISTINCT ON (e."user")
    e."user",
    e.risk_profile,
    e.block_timestamp as enabled_at,
    d.block_timestamp as disabled_at,
    CASE 
        WHEN d.id IS NULL OR e.block_timestamp > d.block_timestamp THEN true
        ELSE false
    END as is_enabled
FROM auto_rebalance_enableds e
LEFT JOIN auto_rebalance_disableds d ON e."user" = d."user" AND d.block_timestamp > e.block_timestamp
ORDER BY e."user", e.block_timestamp DESC;

CREATE UNIQUE INDEX idx_active_auto_rebalance_users_user ON active_auto_rebalance_users("user");
CREATE INDEX idx_active_auto_rebalance_users_enabled ON active_auto_rebalance_users(is_enabled);

-- Create view for user balances (deposits - withdrawals)
CREATE VIEW user_balances AS
SELECT 
    u."user",
    COALESCE(SUM(d.amount), 0) - COALESCE(SUM(w.amount), 0) as balance,
    MAX(d.block_timestamp) as last_deposit_time,
    MAX(w.block_timestamp) as last_withdraw_time
FROM (
    SELECT DISTINCT "user" FROM depositeds
    UNION
    SELECT DISTINCT "user" FROM withdrawns
) u
LEFT JOIN depositeds d ON u."user" = d."user"
LEFT JOIN withdrawns w ON u."user" = w."user"
GROUP BY u."user";

-- Function to refresh the materialized view
CREATE OR REPLACE FUNCTION refresh_active_auto_rebalance_users()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY active_auto_rebalance_users;
END;
$$ LANGUAGE plpgsql;

-- Comments for documentation
COMMENT ON TABLE auto_rebalance_enableds IS 'Tracks when users enable auto-rebalancing with their risk profile';
COMMENT ON TABLE auto_rebalance_disableds IS 'Tracks when users disable auto-rebalancing';
COMMENT ON TABLE depositeds IS 'Tracks user deposits to the RebalancerDelegation contract';
COMMENT ON TABLE withdrawns IS 'Tracks user withdrawals from the RebalancerDelegation contract';
COMMENT ON TABLE rebalanceds IS 'Tracks rebalancing operations performed by operators';
COMMENT ON TABLE operator_addeds IS 'Tracks when operators are added to the system';
COMMENT ON TABLE operator_removeds IS 'Tracks when operators are removed from the system';
COMMENT ON MATERIALIZED VIEW active_auto_rebalance_users IS 'Shows current status of users with auto-rebalance (enabled/disabled)';
COMMENT ON VIEW user_balances IS 'Calculates current balance for each user (deposits - withdrawals)';
