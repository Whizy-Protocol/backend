use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

use super::bet::BetService;
use super::blockchain_sync::BlockchainSyncService;
use super::protocol::ProtocolService;

#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub scheduler_interval_secs: u64,
    pub enable_scheduler: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            scheduler_interval_secs: std::env::var("SCHEDULER_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
            enable_scheduler: std::env::var("ENABLE_SCHEDULER")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
        }
    }
}

pub struct Scheduler {
    pool: PgPool,
    config: SchedulerConfig,
}

impl Scheduler {
    pub fn new(pool: PgPool) -> Self {
        let config = SchedulerConfig::default();
        info!("Scheduler configuration: {:?}", config);
        Self { pool, config }
    }

    pub async fn start(self: Arc<Self>) {
        info!("🚀 Starting scheduler with background jobs");
        info!(
            "   - Background processing: {} (interval: {}s)",
            if self.config.enable_scheduler {
                "enabled"
            } else {
                "disabled"
            },
            self.config.scheduler_interval_secs
        );

        if self.config.enable_scheduler {
            let interval_secs = self.config.scheduler_interval_secs;
            let scheduler = Arc::clone(&self);
            tokio::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(interval_secs));
                let mut sync_count = 0u64;

                loop {
                    interval.tick().await;
                    sync_count += 1;
                    info!(
                        "🔄 [Processing Job #{}] Running background sync",
                        sync_count
                    );

                    let db = crate::db::Database::from_pool(scheduler.pool.clone());

                    let protocol_service = ProtocolService::new(db.clone());
                    let rpc_url = std::env::var("HEDERA_RPC_URL")
                        .unwrap_or_else(|_| "https://hashscan.io/testnet".to_string());

                    match protocol_service
                        .update_all_apys_from_blockchain(&rpc_url)
                        .await
                    {
                        Ok(count) => {
                            info!(
                                "✅ [Processing Job #{}] Updated {} protocol APYs",
                                sync_count, count
                            );
                        }
                        Err(e) => {
                            error!(
                                "❌ [Processing Job #{}] Failed to update protocol APYs: {}",
                                sync_count, e
                            );
                        }
                    }

                    let sync_service = BlockchainSyncService::new(db.clone());
                    match sync_service.run_full_sync().await {
                        Ok(_) => {
                            info!(
                                "✅ [Processing Job #{}] Blockchain sync completed",
                                sync_count
                            );
                        }
                        Err(e) => {
                            error!(
                                "❌ [Processing Job #{}] Blockchain sync failed: {}",
                                sync_count, e
                            );
                        }
                    }

                    let bet_service = BetService::new(db.clone());
                    match bet_service.sync_bets_from_indexer().await {
                        Ok(count) => {
                            if count > 0 {
                                info!(
                                    "✅ [Processing Job #{}] Synced {} bets from indexer and updated markets",
                                    sync_count, count
                                );
                            }
                        }
                        Err(e) => {
                            error!(
                                "❌ [Processing Job #{}] Failed to sync bets from indexer: {}",
                                sync_count, e
                            );
                        }
                    }

                    info!("✅ [Processing Job #{}] Completed successfully", sync_count);
                }
            });
            info!(
                "✅ Background processing job started (every {}s)",
                interval_secs
            );
        } else {
            warn!("⚠️  Background processing is disabled");
        }

        info!("✨ Scheduler started successfully - background jobs running");
    }
}
