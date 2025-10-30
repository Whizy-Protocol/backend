use crate::{
    db::Database,
    error::{AppError, Result},
};
use ethers::prelude::*;
use std::sync::Arc;
use tracing::{error, info, warn};

abigen!(
    WhizyPredictionMarket,
    r#"[
        function createMarket(string calldata question, uint256 endTime, address token) external returns (uint256)
        function markets(uint256) external view returns (uint256 id, string question, uint256 endTime, address token, address vault, uint256 totalYesShares, uint256 totalNoShares, bool resolved, bool outcome, uint8 status)
        function nextMarketId() external view returns (uint256)
        event MarketCreated(uint256 indexed marketId, string question, uint256 endTime, address token, address vault)
    ]"#,
);

pub struct BlockchainSyncService {
    db: Database,
}

impl BlockchainSyncService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn run_full_sync(&self) -> Result<()> {
        Ok(())
    }

    pub async fn sync_markets_to_blockchain(
        &self,
        contract_address: &str,
        rpc_url: &str,
        private_key: &str,
        usdc_address: &str,
        chain_id: u64,
    ) -> Result<usize> {
        info!("🔄 Starting blockchain sync for markets...");

        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| AppError::Internal(format!("Failed to connect to RPC: {}", e)))?;

        let wallet: LocalWallet = private_key
            .parse::<LocalWallet>()
            .map_err(|e| AppError::Internal(format!("Invalid private key: {}", e)))?
            .with_chain_id(chain_id);

        let client = SignerMiddleware::new(provider, wallet);
        let client = Arc::new(client);

        let address: Address = contract_address
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid contract address: {}", e)))?;

        let contract = WhizyPredictionMarket::new(address, client);

        let markets = sqlx::query!(
            r#"
            SELECT id, "marketId", question, description, "endDate"
            FROM markets_extended
            WHERE "blockchainMarketId" IS NULL
            AND status = 'active'
            ORDER BY "createdAt" ASC
            "#
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch markets: {}", e)))?;

        if markets.is_empty() {
            info!("✅ No markets to sync");
            return Ok(0);
        }

        info!("📋 Found {} markets to sync to blockchain", markets.len());

        let token_address: Address = usdc_address
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid token address: {}", e)))?;

        let next_market_id = contract
            .next_market_id()
            .call()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get next market ID: {}", e)))?;
        let total_blockchain_markets = next_market_id.as_u64();

        info!(
            "📊 Blockchain currently has {} markets",
            total_blockchain_markets
        );

        let mut synced_count = 0;

        for market in markets {
            let question = market.question.unwrap_or_default();
            let end_date = market.endDate;

            let end_time = end_date.and_utc().timestamp() as u64;

            let mut found_existing = false;
            for blockchain_id in 0..total_blockchain_markets {
                if let Ok(blockchain_market) =
                    contract.markets(U256::from(blockchain_id)).call().await
                {
                    let blockchain_question = blockchain_market.1.trim();
                    let db_question = question.trim();

                    let is_match = blockchain_question == db_question
                        || db_question.starts_with(blockchain_question);

                    if is_match {
                        info!(
                            "⏭️  Market '{}' already exists on blockchain with ID {}. Updating database...",
                            question.chars().take(60).collect::<String>(),
                            blockchain_id
                        );

                        match sqlx::query!(
                            r#"
                            UPDATE markets_extended
                            SET "blockchainMarketId" = $1, "updatedAt" = CURRENT_TIMESTAMP
                            WHERE id = $2 AND "blockchainMarketId" IS NULL
                            "#,
                            blockchain_id as i64,
                            market.id
                        )
                        .execute(self.db.pool())
                        .await
                        {
                            Ok(result) => {
                                if result.rows_affected() > 0 {
                                    info!(
                                        "✅ Updated market '{}' with existing blockchain ID: {}",
                                        question, blockchain_id
                                    );
                                    synced_count += 1;
                                }
                            }
                            Err(e) => {
                                error!("❌ Failed to update DB for market '{}': {}", question, e);
                            }
                        }

                        found_existing = true;
                        break;
                    }
                }
            }

            if found_existing {
                continue;
            }

            let current_next_id = contract.next_market_id().call().await.map_err(|e| {
                AppError::Internal(format!("Failed to get current market ID: {}", e))
            })?;
            let next_blockchain_id = current_next_id.as_u64();

            let check_existing_id = sqlx::query!(
                r#"SELECT id, question FROM markets_extended WHERE "blockchainMarketId" = $1"#,
                next_blockchain_id as i64
            )
            .fetch_optional(self.db.pool())
            .await
            .map_err(|e| {
                AppError::Internal(format!("Failed to check existing blockchain ID: {}", e))
            })?;

            if let Some(existing) = check_existing_id {
                warn!(
                    "⚠️  Blockchain ID {} already assigned to market {} ('{}'). Skipping creation of '{}'",
                    next_blockchain_id,
                    existing.id,
                    existing.question.unwrap_or_default().chars().take(50).collect::<String>(),
                    question.chars().take(50).collect::<String>()
                );
                continue;
            }

            info!("📝 Creating NEW market on blockchain: {}", question);

            match contract
                .create_market(question.clone(), U256::from(end_time), token_address)
                .send()
                .await
            {
                Ok(pending_tx) => {
                    info!("⏳ Transaction sent: {:?}", pending_tx.tx_hash());

                    match pending_tx.await {
                        Ok(Some(receipt)) => {
                            info!(
                                "✅ Transaction confirmed in block: {}",
                                receipt.block_number.unwrap_or_default()
                            );

                            let mut blockchain_market_id: Option<u64> = None;

                            for log in receipt.logs {
                                if let Ok(event) = contract.decode_event::<MarketCreatedFilter>(
                                    "MarketCreated",
                                    log.topics,
                                    log.data,
                                ) {
                                    blockchain_market_id = Some(event.market_id.as_u64());
                                    break;
                                }
                            }

                            if let Some(market_id) = blockchain_market_id {
                                info!("🔍 Verifying market ID {} on blockchain...", market_id);

                                match contract.markets(U256::from(market_id)).call().await {
                                    Ok(blockchain_market) => {
                                        let blockchain_question = blockchain_market.1;

                                        if blockchain_question.trim() != question.trim() {
                                            warn!(
                                                "⚠️  Question truncated by blockchain! Database: '{}' -> Blockchain: '{}'",
                                                question.chars().take(100).collect::<String>(),
                                                blockchain_question
                                            );
                                            warn!("   Updating database to match blockchain question...");

                                            if let Err(e) = sqlx::query!(
                                                r#"UPDATE markets_extended SET question = $1, "updatedAt" = CURRENT_TIMESTAMP WHERE id = $2"#,
                                                blockchain_question,
                                                market.id
                                            )
                                            .execute(self.db.pool())
                                            .await {
                                                error!("Failed to update question in database: {}", e);
                                            } else {
                                                info!("   ✅ Database question updated to match blockchain");
                                            }
                                        } else {
                                            info!("✅ Verified market {} on blockchain with matching question", market_id);
                                        }
                                    }
                                    Err(e) => {
                                        error!(
                                            "❌ Failed to verify market {} on blockchain: {}",
                                            market_id, e
                                        );
                                        return Ok(synced_count);
                                    }
                                }

                                let update_result = sqlx::query(
                                    r#"
                                    UPDATE markets_extended
                                    SET "blockchainMarketId" = $1, "updatedAt" = CURRENT_TIMESTAMP
                                    WHERE id = $2 AND "blockchainMarketId" IS NULL
                                    "#,
                                )
                                .bind(market_id as i64)
                                .bind(&market.id)
                                .execute(self.db.pool())
                                .await;

                                match update_result {
                                    Ok(result) => {
                                        if result.rows_affected() > 0 {
                                            info!(
                                                "🎉 Market '{}' synced with blockchain ID: {}",
                                                question, market_id
                                            );
                                            synced_count += 1;

                                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                                100,
                                            ))
                                            .await;
                                        } else {
                                            warn!("⚠️  Market '{}' was already updated or no longer eligible", question);
                                        }
                                    }
                                    Err(e) => {
                                        let error_msg = e.to_string();
                                        if error_msg.contains("duplicate key")
                                            || error_msg.contains("blockchainMarketId_key")
                                        {
                                            warn!(
                                                "⚠️  Blockchain ID {} already taken (race condition). Market '{}' will be retried later.",
                                                market_id, question
                                            );
                                        } else {
                                            error!(
                                                "❌ Failed to update DB for market '{}': {}",
                                                question, e
                                            );
                                        }
                                    }
                                }
                            } else {
                                error!("❌ Failed to parse MarketCreated event from transaction receipt");
                            }
                        }
                        Ok(None) => {
                            error!("❌ Transaction returned no receipt");
                        }
                        Err(e) => {
                            error!("❌ Transaction failed for '{}': {}", question, e);
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Failed to send transaction for '{}': {}", question, e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }

        info!(
            "✅ Blockchain sync complete - {} markets synced",
            synced_count
        );
        Ok(synced_count)
    }

    pub async fn verify_blockchain_sync(
        &self,
        contract_address: &str,
        rpc_url: &str,
    ) -> Result<usize> {
        info!("🔍 Verifying all markets against blockchain...");

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
        let total_markets = next_market_id.as_u64();

        info!("📊 Found {} markets on blockchain", total_markets);

        let mut verified_count = 0;
        let created_count = 0;

        for market_id in 0..total_markets {
            match contract.markets(U256::from(market_id)).call().await {
                Ok(blockchain_market) => {
                    let question = blockchain_market.1;

                    let existing = sqlx::query!(
                        r#"SELECT id FROM markets_extended WHERE "blockchainMarketId" = $1"#,
                        market_id as i64
                    )
                    .fetch_optional(self.db.pool())
                    .await
                    .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

                    if existing.is_some() {
                        verified_count += 1;
                    } else {
                        let existing_by_question = sqlx::query!(
                            r#"SELECT id, "blockchainMarketId" FROM markets_extended WHERE question = $1"#,
                            question
                        )
                        .fetch_optional(self.db.pool())
                        .await
                        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

                        if let Some(market) = existing_by_question {
                            if market.blockchainMarketId.is_none() {
                                let update_result = sqlx::query!(
                                    r#"UPDATE markets_extended SET "blockchainMarketId" = $1, "updatedAt" = CURRENT_TIMESTAMP WHERE id = $2"#,
                                    market_id as i64,
                                    market.id
                                )
                                .execute(self.db.pool())
                                .await
                                .map_err(|e| AppError::Internal(format!("Failed to update market: {}", e)))?;

                                info!(
                                    "✅ Updated existing market '{}' (ID: {}) with blockchain ID {} (rows affected: {})",
                                    question.chars().take(60).collect::<String>(),
                                    market.id,
                                    market_id,
                                    update_result.rows_affected()
                                );
                            } else {
                                info!(
                                    "ℹ️  Market '{}' already has blockchain ID {}, skipping",
                                    question.chars().take(60).collect::<String>(),
                                    market.blockchainMarketId.unwrap_or(-1)
                                );
                            }
                            verified_count += 1;
                        } else {
                            info!(
                                "⏭️  Skipping blockchain market ID {} (not in database): '{}'",
                                market_id,
                                question.chars().take(60).collect::<String>()
                            );
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Failed to read market {}: {}", market_id, e);
                }
            }
        }

        info!(
            "✅ Verification complete: {} verified, {} created from blockchain",
            verified_count, created_count
        );
        Ok(created_count)
    }
}
