-- Rollback: Remove MarketVaultRebalanced event table
-- Description: Drops the market_vault_rebalanceds table and related views
-- Date: 2025-01-31

-- Drop view first
DROP VIEW IF EXISTS market_rebalancing_history;

-- Drop table and its indexes
DROP TABLE IF EXISTS market_vault_rebalanceds;
