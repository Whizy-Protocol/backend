use axum::{
    extract::State,
    middleware,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::json;
use sqlx::Row;

use crate::{db::Database, error::AppError, middleware::auth::require_api_key};

pub fn create_admin_router() -> Router<(Database, crate::config::Config)> {
    Router::new()
        .route("/stats", get(get_admin_stats))
        .route("/users", get(list_all_users))
        .route("/sync/trigger", post(trigger_admin_sync))
        .route("/sync/blockchain", post(trigger_blockchain_sync))
        .route_layer(middleware::from_fn(require_api_key))
}

async fn get_admin_stats(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats = sqlx::query!(
        r#"
        SELECT
            (SELECT COUNT(*) FROM users) as total_users,
            (SELECT COUNT(*) FROM markets_extended) as total_markets,
            (SELECT COUNT(*) FROM bets_extended) as total_bets,
            (SELECT COUNT(*) FROM protocols) as total_protocols,
            (SELECT COALESCE(SUM(volume), 0) FROM markets_extended) as total_volume
        "#
    )
    .fetch_one(db.pool())
    .await?;

    Ok(Json(json!({
        "data": {
            "totalUsers": stats.total_users,
            "totalMarkets": stats.total_markets,
            "totalBets": stats.total_bets,
            "totalProtocols": stats.total_protocols,
            "totalVolume": stats.total_volume.unwrap_or_default().to_string()
        }
    })))
}

async fn list_all_users(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let users = sqlx::query(
        r#"
        SELECT id, address, username, "avatarUrl", "createdAt"
        FROM users
        ORDER BY "createdAt" DESC
        LIMIT 100
        "#,
    )
    .fetch_all(db.pool())
    .await?;

    let user_list: Vec<serde_json::Value> = users.iter().map(|row| {
        json!({
            "id": row.try_get::<String, _>("id").unwrap_or_default(),
            "address": row.try_get::<String, _>("address").unwrap_or_default(),
            "username": row.try_get::<Option<String>, _>("username").unwrap_or(None),
            "avatarUrl": row.try_get::<Option<String>, _>("avatarUrl").unwrap_or(None),
            "createdAt": row.try_get::<chrono::NaiveDateTime, _>("createdAt").ok().map(|d| d.to_string())
        })
    }).collect();

    Ok(Json(json!({
        "data": user_list
    })))
}

async fn trigger_admin_sync(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sync_service = crate::services::SyncService::new(db);
    sync_service.full_sync().await?;

    Ok(Json(json!({
        "message": "Admin sync triggered successfully"
    })))
}

async fn trigger_blockchain_sync(
    State((db, config)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sync_service = crate::services::SyncService::new(db);
    let synced_count = sync_service
        .sync_from_blockchain(&config.whizy_prediction_market_addr, &config.base_rpc_url)
        .await?;

    Ok(Json(json!({
        "message": format!("Blockchain sync completed - {} markets synced", synced_count),
        "synced_count": synced_count
    })))
}
