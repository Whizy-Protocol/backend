-- Rollback: Remove RebalancerDelegation event tables
-- Description: Drops all tables and views created for RebalancerDelegation contract events
-- Date: 2025-01-31

-- Drop function
DROP FUNCTION IF EXISTS refresh_active_auto_rebalance_users();

-- Drop views
DROP VIEW IF EXISTS user_balances;
DROP MATERIALIZED VIEW IF EXISTS active_auto_rebalance_users;

-- Drop tables
DROP TABLE IF EXISTS operator_removeds;
DROP TABLE IF EXISTS operator_addeds;
DROP TABLE IF EXISTS rebalanceds;
DROP TABLE IF EXISTS withdrawns;
DROP TABLE IF EXISTS depositeds;
DROP TABLE IF EXISTS auto_rebalance_disableds;
DROP TABLE IF EXISTS auto_rebalance_enableds;
