use anyhow::Result;
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use whizy_base_server::{config::Config, db::Database, routes, seed, services::*};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "whizy_base_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("🚀 Starting Whizy HEDERA Testnet Backend Server");

    let config = Config::from_env()?;
    info!("✅ Configuration loaded");
    info!(
        "   Network: HEDERA Testnet (Chain ID: {})",
        config.base_chain_id
    );
    info!("   RPC: {}", config.base_rpc_url);
    info!(
        "   WhizyPredictionMarket: {}",
        config.whizy_prediction_market_addr
    );
    info!("   ProtocolSelector: {}", config.protocol_selector_addr);

    info!("📦 Connecting to database...");
    let db = Database::new(&config.database_url).await?;
    info!("✅ Connected to database");

    info!("🔄 Running database migrations...");
    sqlx::migrate!("./migrations").run(db.pool()).await.ok();
    info!("✅ Migrations completed");

    if std::env::var("RUN_SEEDS").unwrap_or_default() == "true" {
        info!("🌱 Running database seeds...");
        seed::run_all_seeds(db.pool()).await?;
        info!("✅ Seeds completed");

        if let Ok(private_key) = std::env::var("PRIVATE_KEY") {
            info!("🔗 Auto-syncing markets to blockchain...");
            let blockchain_sync = BlockchainSyncService::new(db.clone());

            match blockchain_sync
                .sync_markets_to_blockchain(
                    &config.whizy_prediction_market_addr,
                    &config.base_rpc_url,
                    &private_key,
                    &config.usdc_address,
                    config.base_chain_id,
                )
                .await
            {
                Ok(count) => info!("✅ Successfully synced {} markets to blockchain", count),
                Err(e) => error!(
                    "⚠️  Failed to sync markets to blockchain: {}. Continuing...",
                    e
                ),
            }
        } else {
            info!("ℹ️  PRIVATE_KEY not set. Skipping blockchain sync. Run 'cargo run --bin sync_markets' to sync manually.");
        }
    }

    info!("🚀 Starting background scheduler...");
    let scheduler = std::sync::Arc::new(Scheduler::new(db.pool().clone()));
    scheduler.start().await;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest("/api", routes::create_routes(db.clone(), config.clone()))
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🌐 Server listening on http://{}", addr);
    info!("📚 API documentation: http://{}/api", addr);
    info!("💚 Health check: http://{}/api/health", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
