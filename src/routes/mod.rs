use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;

use crate::{config::Config, db::Database, error::AppError, models::*, services::*};

mod auth;
mod bets;
mod blockchain;
mod charts;
mod markets;
mod prices;
mod protocols;
mod sync;
mod yields;

pub use auth::create_auth_router;
pub use bets::create_bets_router;
pub use blockchain::create_blockchain_router;
pub use charts::create_charts_router;
pub use markets::create_markets_router;
pub use prices::create_prices_router;
pub use protocols::create_protocols_router;
pub use sync::create_sync_router;
pub use yields::create_yields_router;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub market_service: Arc<MarketService>,
    pub bet_service: Arc<BetService>,
    pub user_service: Arc<UserService>,
    pub protocol_service: Arc<ProtocolService>,
    pub stats_service: Arc<StatsService>,
    pub sync_service: Arc<SyncService>,
}

pub fn create_routes(db: Database, config: Config) -> Router {
    let state = AppState {
        config: config.clone(),
        market_service: Arc::new(MarketService::new(db.clone())),
        bet_service: Arc::new(BetService::new(db.clone())),
        user_service: Arc::new(UserService::new(db.clone())),
        protocol_service: Arc::new(ProtocolService::new(db.clone())),
        stats_service: Arc::new(StatsService::new(db.clone())),
        sync_service: Arc::new(SyncService::new(db.clone())),
    };

    let shared_state = (db.clone(), config.clone());

    Router::new()
        .route("/", get(api_info))
        .route("/health", get(health_check))
        .nest(
            "/auth",
            create_auth_router().with_state(shared_state.clone()),
        )
        .nest(
            "/markets",
            create_markets_router().with_state(shared_state.clone()),
        )
        .nest(
            "/bets",
            create_bets_router().with_state(shared_state.clone()),
        )
        .nest(
            "/charts",
            create_charts_router().with_state(shared_state.clone()),
        )
        .nest(
            "/protocols",
            create_protocols_router().with_state(shared_state.clone()),
        )
        .nest(
            "/sync",
            create_sync_router().with_state(shared_state.clone()),
        )
        .nest(
            "/yields",
            create_yields_router().with_state(shared_state.clone()),
        )
        .nest(
            "/prices",
            create_prices_router().with_state(shared_state.clone()),
        )
        .nest(
            "/blockchain",
            create_blockchain_router().with_state(shared_state.clone()),
        )
        .nest(
            "/admin",
            crate::admin::create_admin_router().with_state(shared_state.clone()),
        )
        .route("/users/:address", get(get_user))
        .route("/users/:address/bets", get(get_user_bets))
        .route("/users/:address/stats", get(get_user_stats))
        .route("/stats/platform", get(get_platform_stats))
        .route("/stats/leaderboard", get(get_leaderboard))
        .with_state(state)
}

async fn api_info(State(state): State<AppState>) -> Result<Json<ApiInfoResponse>, AppError> {
    Ok(Json(ApiInfoResponse {
        name: "Whizy HEDERA Testnet Backend".to_string(),
        version: "1.0.0".to_string(),
        network: "HEDERA Testnet".to_string(),
        chain_id: state.config.base_chain_id,
        contracts: ContractAddresses {
            prediction_market: state.config.whizy_prediction_market_addr,
            protocol_selector: state.config.protocol_selector_addr,
        },
    }))
}

async fn health_check() -> Result<Json<HealthResponse>, AppError> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        database: "connected".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

async fn get_user(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<User>, AppError> {
    let user = state.user_service.get_user_by_address(&address).await?;
    Ok(Json(user))
}

async fn get_user_bets(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(params): Query<BetQueryParams>,
) -> Result<Json<BetResponse>, AppError> {
    let response = state.bet_service.get_bets_by_user(&address, params).await?;
    Ok(Json(response))
}

async fn get_user_stats(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<UserStats>, AppError> {
    let stats = state.user_service.get_user_stats(&address).await?;
    Ok(Json(stats))
}

async fn get_platform_stats(
    State(state): State<AppState>,
) -> Result<Json<PlatformStats>, AppError> {
    let stats = state.stats_service.get_platform_stats().await?;
    Ok(Json(stats))
}

async fn get_leaderboard(State(state): State<AppState>) -> Result<Json<Vec<UserStats>>, AppError> {
    let leaderboard = state.stats_service.get_leaderboard(100).await?;
    Ok(Json(leaderboard))
}
