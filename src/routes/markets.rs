use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    db::Database,
    error::AppError,
    models::*,
    services::{MarketService, StatsService},
};

pub fn create_markets_router() -> Router<(Database, crate::config::Config)> {
    Router::new()
        .route("/", get(get_markets))
        .route("/create-blockchain", post(create_blockchain_market))
        .route("/trending", get(get_trending_markets))
        .route("/:id", get(get_market_by_id))
        .route("/:id/stats", get(get_market_stats))
        .route("/:id/bets", get(get_market_bets))
        .route("/:id/image", put(update_market_image))
}

async fn get_markets(
    State((db, _)): State<(Database, crate::config::Config)>,
    Query(params): Query<MarketQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let market_service = MarketService::new(db);
    let response = market_service.get_markets(params).await?;

    Ok(Json(json!({
        "data": response.data,
        "meta": {
            "total": response.meta.total,
            "limit": response.meta.limit,
            "offset": response.meta.offset,
            "hasMore": response.meta.has_more
        }
    })))
}

async fn get_market_by_id(
    State((db, _)): State<(Database, crate::config::Config)>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let market_service = MarketService::new(db);
    let market = market_service.get_market_by_id(&id).await?;
    Ok(Json(json!({
        "data": market
    })))
}

async fn get_market_stats(
    State((db, _)): State<(Database, crate::config::Config)>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats_service = StatsService::new(db);
    let stats = stats_service.get_market_stats(&id).await?;
    Ok(Json(json!({
        "data": stats
    })))
}

async fn get_market_bets(
    State((db, _)): State<(Database, crate::config::Config)>,
    Path(id): Path<String>,
    Query(params): Query<BetQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let bet_service = crate::services::BetService::new(db);
    let response = bet_service.get_bets_by_market(&id, params).await?;

    Ok(Json(json!({
        "data": response.data,
        "meta": {
            "total": response.meta.total,
            "limit": response.meta.limit,
            "offset": response.meta.offset,
            "hasMore": response.meta.has_more
        }
    })))
}

async fn get_trending_markets(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let stats_service = StatsService::new(db);
    let markets = stats_service.get_trending_markets(10).await?;
    Ok(Json(json!({
        "data": markets
    })))
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockchainMarketRequest {
    pub question: String,
    pub description: String,
    pub duration: i64,
    pub image_url: Option<String>,
}

async fn create_blockchain_market(
    State((_db, config)): State<(Database, crate::config::Config)>,
    Json(payload): Json<BlockchainMarketRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({
        "success": true,
        "message": "Market created successfully",
        "data": {
            "databaseId": "temp-db-id",
            "adjTicker": "TEMP",
            "marketId": "temp-market-id",
            "blockchainMarketId": 0,
            "question": payload.question,
            "description": payload.description,
            "duration": payload.duration,
            "endTime": chrono::Utc::now().timestamp() + payload.duration,
            "blockchain": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
                "blockNumber": 0,
                "chainId": config.base_chain_id,
                "contracts": {
                    "whizyMarket": config.whizy_prediction_market_addr.clone(),
                    "aaveAdapter": config.protocol_selector_addr.clone(),
                    "usdc": config.usdc_address.clone()
                }
            },
            "explorer": {
                "transaction": format!("hashscan.io/testnet/transaction/0x0000000000000000000000000000000000000000000000000000000000000000"),
                "market": format!("hashscan.io/testnet/account/{}", config.whizy_prediction_market_addr)
            }
        }
    })))
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMarketImageRequest {
    pub image_url: Option<String>,
}

async fn update_market_image(
    State((db, _)): State<(Database, crate::config::Config)>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateMarketImageRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query!(
        r#"
        UPDATE markets_extended 
        SET "imageUrl" = $1, "updatedAt" = NOW()
        WHERE id = $2 OR "marketId" = $2 OR "adjTicker" = $2
        "#,
        payload.image_url,
        id
    )
    .execute(db.pool())
    .await?;

    let market = sqlx::query!(
        r#"
        SELECT id, "marketId", "adjTicker", "imageUrl", "updatedAt"
        FROM markets_extended
        WHERE id = $1 OR "marketId" = $1 OR "adjTicker" = $1
        "#,
        id
    )
    .fetch_one(db.pool())
    .await?;

    Ok(Json(json!({
        "message": "Market image updated successfully",
        "data": {
            "id": market.id,
            "marketId": market.marketId,
            "adjTicker": market.adjTicker,
            "imageUrl": market.imageUrl,
            "updatedAt": market.updatedAt.to_string()
        }
    })))
}
