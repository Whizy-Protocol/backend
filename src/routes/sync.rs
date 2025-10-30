use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::json;

use crate::{db::Database, error::AppError, models::SyncStatusResponse, services::SyncService};

pub fn create_sync_router() -> Router<(Database, crate::config::Config)> {
    Router::new()
        .route("/status", get(sync_status))
        .route("/full", post(trigger_full_sync))
        .route("/blockchain", post(sync_from_blockchain))
        .route("/market/:id", post(sync_specific_market))
}

async fn sync_status(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<SyncStatusResponse>, AppError> {
    let sync_service = SyncService::new(db);
    let status = sync_service.get_sync_status().await?;
    Ok(Json(status))
}

async fn trigger_full_sync(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sync_service = SyncService::new(db);
    sync_service.full_sync().await?;
    Ok(Json(json!({
        "status": "success",
        "message": "Full sync completed"
    })))
}

async fn sync_from_blockchain(
    State((db, config)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sync_service = SyncService::new(db);
    let synced_count = sync_service
        .sync_from_blockchain(&config.whizy_prediction_market_addr, &config.base_rpc_url)
        .await?;
    Ok(Json(json!({
        "status": "success",
        "message": format!("Synced {} markets from blockchain", synced_count),
        "synced_count": synced_count
    })))
}

async fn sync_specific_market(
    State((db, _)): State<(Database, crate::config::Config)>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sync_service = SyncService::new(db);
    sync_service.sync_market_by_id(&id).await?;
    Ok(Json(json!({
        "status": "success",
        "message": format!("Market {} synced", id)
    })))
}
