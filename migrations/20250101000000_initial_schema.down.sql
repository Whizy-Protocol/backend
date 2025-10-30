-- ================================================
-- Rollback Initial Database Schema Migration
-- ================================================

-- Drop triggers
DROP TRIGGER IF EXISTS sync_winnings_claimed_trigger ON winnings_claimeds;
DROP TRIGGER IF EXISTS sync_bet_placed_trigger ON bet_placeds;
DROP TRIGGER IF EXISTS sync_market_resolved_trigger ON market_resolveds;
DROP TRIGGER IF EXISTS sync_market_created_trigger ON market_createds;

DROP TRIGGER IF EXISTS update_sync_status_updated_at ON sync_status;
DROP TRIGGER IF EXISTS update_blockchain_events_updated_at ON blockchain_events;
DROP TRIGGER IF EXISTS update_bets_extended_updated_at ON bets_extended;
DROP TRIGGER IF EXISTS update_markets_extended_updated_at ON markets_extended;
DROP TRIGGER IF EXISTS update_protocols_updated_at ON protocols;
DROP TRIGGER IF EXISTS update_users_updated_at ON users;

-- Drop event tables
DROP TABLE IF EXISTS winnings_claimeds CASCADE;
DROP TABLE IF EXISTS unpauseds CASCADE;
DROP TABLE IF EXISTS protocol_updateds CASCADE;
DROP TABLE IF EXISTS protocol_registereds CASCADE;
DROP TABLE IF EXISTS pauseds CASCADE;
DROP TABLE IF EXISTS ownership_transferreds CASCADE;
DROP TABLE IF EXISTS market_resolveds CASCADE;
DROP TABLE IF EXISTS market_createds CASCADE;
DROP TABLE IF EXISTS bet_placeds CASCADE;
DROP TABLE IF EXISTS auto_withdraw_executeds CASCADE;
DROP TABLE IF EXISTS auto_deposit_executeds CASCADE;

-- Drop core tables
DROP TABLE IF EXISTS sync_status CASCADE;
DROP TABLE IF EXISTS sync_states CASCADE;
DROP TABLE IF EXISTS yield_records CASCADE;
DROP TABLE IF EXISTS user_yields CASCADE;
DROP TABLE IF EXISTS fee_records CASCADE;
DROP TABLE IF EXISTS blockchain_events CASCADE;
DROP TABLE IF EXISTS bets_extended CASCADE;
DROP TABLE IF EXISTS markets_extended CASCADE;
DROP TABLE IF EXISTS protocols CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- Drop functions
DROP FUNCTION IF EXISTS sync_winnings_claimed();
DROP FUNCTION IF EXISTS sync_bet_placed();
DROP FUNCTION IF EXISTS sync_market_resolved();
DROP FUNCTION IF EXISTS sync_market_created();
DROP FUNCTION IF EXISTS update_updated_at_column();
