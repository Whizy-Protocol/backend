-- Rollback Migration: Restore quoted user column names
-- Description: Reverts the user column name changes back to quoted format
-- Date: 2025-01-31

-- Drop views first
DROP MATERIALIZED VIEW IF EXISTS active_auto_rebalance_users CASCADE;
DROP VIEW IF EXISTS user_balances CASCADE;

-- Rename columns back to "user" (with quotes)
ALTER TABLE auto_rebalance_enableds RENAME COLUMN user TO "user";
ALTER TABLE auto_rebalance_disableds RENAME COLUMN user TO "user";
ALTER TABLE depositeds RENAME COLUMN user TO "user";
ALTER TABLE withdrawns RENAME COLUMN user TO "user";
ALTER TABLE rebalanceds RENAME COLUMN user TO "user";

-- Recreate indexes with quotes
DROP INDEX IF EXISTS idx_auto_rebalance_enableds_user;
DROP INDEX IF EXISTS idx_auto_rebalance_disableds_user;
DROP INDEX IF EXISTS idx_depositeds_user;
DROP INDEX IF EXISTS idx_withdrawns_user;
DROP INDEX IF EXISTS idx_rebalanceds_user;

CREATE INDEX idx_auto_rebalance_enableds_user ON auto_rebalance_enableds("user");
CREATE INDEX idx_auto_rebalance_disableds_user ON auto_rebalance_disableds("user");
CREATE INDEX idx_depositeds_user ON depositeds("user");
CREATE INDEX idx_withdrawns_user ON withdrawns("user");
CREATE INDEX idx_rebalanceds_user ON rebalanceds("user");

-- Recreate materialized view with quotes
CREATE MATERIALIZED VIEW active_auto_rebalance_users AS
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

-- Recreate view with quotes
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

-- Recreate refresh function
CREATE OR REPLACE FUNCTION refresh_active_auto_rebalance_users()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY active_auto_rebalance_users;
END;
$$ LANGUAGE plpgsql;
