use crate::{
    db::Database,
    error::{AppError, Result},
    models::SyncStatusResponse,
    services::{BetService, MarketService, ProtocolService},
};
use ethers::prelude::*;
use sqlx::Row;
use std::sync::Arc;
use tracing::{error, info, warn};

abigen!(
    WhizyPredictionMarket,
    r#"[
        function markets(uint256) external view returns (uint256 id, string question, string description, uint256 bettingDeadline, uint256 resolutionDeadline, address tokenAddress, uint256 minBet, uint256 maxBet, bool resolved, bool outcome, uint256 totalYesAmount, uint256 totalNoAmount, uint256 totalYieldEarned, address yieldProtocolAddr, address protocolSelectorAddr, uint8 status)
        function nextMarketId() external view returns (uint256)
    ]"#,
);

pub struct SyncService {
    db: Database,
    market_service: MarketService,
    bet_service: BetService,
    protocol_service: ProtocolService,
}

impl SyncService {
    pub fn new(db: Database) -> Self {
        let market_service = MarketService::new(db.clone());
        let bet_service = BetService::new(db.clone());
        let protocol_service = ProtocolService::new(db.clone());

        Self {
            db,
            market_service,
            bet_service,
            protocol_service,
        }
    }

    pub async fn get_sync_status(&self) -> Result<SyncStatusResponse> {
        let last_synced_block = 0;

        let markets_synced: i64 = sqlx::query("SELECT COUNT(*) as count FROM markets_extended")
            .fetch_one(self.db.pool())
            .await?
            .try_get("count")?;

        let bets_synced: i64 = sqlx::query("SELECT COUNT(*) as count FROM bets_extended")
            .fetch_one(self.db.pool())
            .await?
            .try_get("count")?;

        let protocols_synced: i64 = sqlx::query("SELECT COUNT(*) as count FROM protocols")
            .fetch_one(self.db.pool())
            .await?
            .try_get("count")?;

        Ok(SyncStatusResponse {
            last_synced_block,
            is_syncing: false,
            markets_synced,
            bets_synced,
            protocols_synced,
            last_sync_time: Some(chrono::Utc::now().to_rfc3339()),
        })
    }

    pub async fn full_sync(&self) -> Result<()> {
        info!("Starting full sync from indexer to backend database");

        self.sync_protocols().await?;

        self.sync_markets().await?;

        self.sync_bets().await?;

        self.sync_market_resolutions().await?;

        info!("Full sync completed successfully");
        Ok(())
    }

    async fn sync_protocols(&self) -> Result<()> {
        info!("Syncing protocols from indexer...");

        let protocols = self.protocol_service.get_indexer_protocols().await?;
        let mut synced_count = 0;

        for protocol in protocols {
            match self
                .protocol_service
                .upsert_protocol_from_indexer(&protocol)
                .await
            {
                Ok(id) => {
                    info!("Synced protocol: {} (id: {})", protocol.name, id);
                    synced_count += 1;
                }
                Err(e) => {
                    error!("Failed to sync protocol {}: {}", protocol.name, e);
                }
            }
        }

        info!("Synced {} protocols", synced_count);
        Ok(())
    }

    async fn sync_markets(&self) -> Result<()> {
        info!("Syncing markets from indexer...");

        let markets = self.market_service.get_indexer_markets().await?;
        let mut synced_count = 0;

        for market in markets {
            match self
                .market_service
                .upsert_market_from_indexer(&market)
                .await
            {
                Ok(id) => {
                    info!("Synced market: {} (id: {})", market.question, id);
                    synced_count += 1;
                }
                Err(e) => {
                    error!("Failed to sync market {}: {}", market.question, e);
                }
            }
        }

        info!("Synced {} markets", synced_count);
        Ok(())
    }

    async fn sync_bets(&self) -> Result<()> {
        info!("Syncing bets from indexer...");

        let bets = self.bet_service.get_indexer_bets().await?;
        let mut synced_count = 0;

        for bet in bets {
            match self.bet_service.upsert_bet_from_indexer(&bet).await {
                Ok(id) => {
                    synced_count += 1;

                    if let Ok(bet_data) = self.bet_service.get_bet_by_id(&id).await {
                        if let Some(market_id) = bet_data.market_id {
                            if let Err(e) = self
                                .market_service
                                .recalculate_market_stats(&market_id)
                                .await
                            {
                                warn!(
                                    "Failed to recalculate market stats for {}: {}",
                                    market_id, e
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to sync bet {}: {}", bet.id, e);
                }
            }
        }

        info!("Synced {} bets", synced_count);
        Ok(())
    }

    async fn sync_market_resolutions(&self) -> Result<()> {
        info!("Syncing market resolutions - no indexer DB available");
        Ok(())
    }

    pub async fn incremental_sync(&self) -> Result<()> {
        info!("Starting incremental sync - no indexer DB available");
        Ok(())
    }

    pub async fn sync_market_by_id(&self, _blockchain_market_id: &str) -> Result<()> {
        info!("Sync specific market - no indexer DB available");
        Ok(())
    }

    pub async fn sync_from_blockchain(
        &self,
        contract_address: &str,
        rpc_url: &str,
    ) -> Result<usize> {
        info!("🔄 Starting sync from blockchain to database...");

        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| AppError::Internal(format!("Failed to connect to RPC: {}", e)))?;
        let client = Arc::new(provider);

        let address: Address = contract_address
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid contract address: {}", e)))?;

        let contract = WhizyPredictionMarket::new(address, client);

        let next_market_id = contract
            .next_market_id()
            .call()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get next market ID: {}", e)))?;
        let total_blockchain_markets = next_market_id.as_u64();

        info!(
            "📊 Found {} markets on blockchain",
            total_blockchain_markets
        );

        if total_blockchain_markets == 0 {
            info!("✅ No markets on blockchain to sync");
            return Ok(0);
        }

        let mut synced_count = 0;

        for blockchain_id in 0..total_blockchain_markets {
            match contract.markets(U256::from(blockchain_id)).call().await {
                Ok(blockchain_market) => {
                    let blockchain_question = blockchain_market.1.trim();

                    info!(
                        "🔍 Checking blockchain market ID {}: '{}'",
                        blockchain_id,
                        blockchain_question.chars().take(60).collect::<String>()
                    );

                    let existing_row = sqlx::query(
                        r#"
                        SELECT id, question 
                        FROM markets_extended 
                        WHERE question = $1 AND "blockchainMarketId" IS NULL
                        LIMIT 1
                        "#,
                    )
                    .bind(blockchain_question)
                    .fetch_optional(self.db.pool())
                    .await
                    .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

                    let existing = if existing_row.is_some() {
                        existing_row
                    } else {
                        let pattern = format!("{}%", blockchain_question);
                        sqlx::query(
                            r#"
                            SELECT id, question 
                            FROM markets_extended 
                            WHERE question LIKE $1 AND "blockchainMarketId" IS NULL
                            LIMIT 1
                            "#,
                        )
                        .bind(&pattern)
                        .fetch_optional(self.db.pool())
                        .await
                        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?
                    };

                    if let Some(row) = existing {
                        let market_id: String = row
                            .try_get("id")
                            .map_err(|e| AppError::Internal(format!("Failed to get id: {}", e)))?;
                        let db_question: Option<String> = row.try_get("question").map_err(|e| {
                            AppError::Internal(format!("Failed to get question: {}", e))
                        })?;

                        let db_question_str = db_question.as_deref().unwrap_or("");
                        let match_type = if db_question_str == blockchain_question {
                            "exact"
                        } else {
                            "prefix"
                        };

                        info!(
                            "🎯 Found {} match for blockchain ID {}",
                            match_type, blockchain_id
                        );
                        info!("   Blockchain: '{}'", blockchain_question);
                        info!(
                            "   Database:   '{}'",
                            db_question_str.chars().take(100).collect::<String>()
                        );

                        let existing_blockchain_id = sqlx::query!(
                            r#"SELECT id FROM markets_extended WHERE "blockchainMarketId" = $1"#,
                            blockchain_id as i64
                        )
                        .fetch_optional(self.db.pool())
                        .await
                        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

                        if let Some(existing) = existing_blockchain_id {
                            if existing.id != market_id {
                                warn!(
                                    "⚠️  Blockchain ID {} already assigned to market {} (trying to assign to {}). Skipping to avoid duplicate.",
                                    blockchain_id, existing.id, market_id
                                );
                                continue;
                            }
                        }

                        match sqlx::query!(
                            r#"
                            UPDATE markets_extended
                            SET "blockchainMarketId" = $1, "updatedAt" = CURRENT_TIMESTAMP
                            WHERE id = $2 AND "blockchainMarketId" IS NULL
                            "#,
                            blockchain_id as i64,
                            market_id
                        )
                        .execute(self.db.pool())
                        .await
                        {
                            Ok(result) => {
                                if result.rows_affected() > 0 {
                                    info!(
                                        "✅ Updated market (DB ID: {}) with blockchain ID {}",
                                        market_id, blockchain_id
                                    );
                                    synced_count += 1;
                                } else {
                                    info!(
                                        "ℹ️  Market {} already has a blockchain ID or was updated by another process",
                                        market_id
                                    );
                                }
                            }
                            Err(e) => {
                                error!(
                                    "❌ Failed to update market '{}': {}",
                                    blockchain_question, e
                                );
                            }
                        }
                    } else {
                        let check = sqlx::query!(
                            r#"
                            SELECT id, "blockchainMarketId" 
                            FROM markets_extended 
                            WHERE question = $1 OR "blockchainMarketId" = $2
                            LIMIT 1
                            "#,
                            blockchain_question,
                            blockchain_id as i64
                        )
                        .fetch_optional(self.db.pool())
                        .await
                        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

                        if let Some(existing_market) = check {
                            if existing_market.blockchainMarketId.is_some() {
                                info!(
                                    "ℹ️  Question '{}' already has blockchain ID {}, skipping",
                                    blockchain_question.chars().take(60).collect::<String>(),
                                    existing_market.blockchainMarketId.unwrap_or(-1)
                                );
                            }
                        } else {
                            info!(
                                "⏭️  No database match for blockchain market ID {}: '{}'",
                                blockchain_id,
                                blockchain_question.chars().take(60).collect::<String>()
                            );
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "❌ Failed to read blockchain market {}: {}",
                        blockchain_id, e
                    );
                }
            }
        }

        info!(
            "✅ Blockchain sync complete - {} markets updated with blockchain IDs",
            synced_count
        );
        Ok(synced_count)
    }
}
