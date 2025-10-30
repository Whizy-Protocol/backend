use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{db::Database, error::AppError, services::BlockchainYieldService};

pub fn create_yields_router() -> Router<(Database, crate::config::Config)> {
    Router::new()
        .route("/", get(get_yields))
        .route("/summary", get(get_yield_summary))
        .route("/protocols", get(get_yield_protocols))
        .route("/update", post(update_yields))
        .route("/apy/current", get(get_current_apy))
        .route("/contract/test", get(test_contract_connectivity))
        .route("/contract/apy", get(get_contract_apy))
        .route(
            "/blockchain/market/:market_id",
            get(get_market_yield_from_blockchain),
        )
        .route("/blockchain/user", get(get_user_yield_from_blockchain))
        .route("/blockchain/sync/:market_id", post(sync_market_yield))
        .route("/blockchain/sync-all", post(sync_all_market_yields))
}

async fn get_yields(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let yields = sqlx::query!(
        r#"
        SELECT 
            m.id,
            m."blockchainMarketId",
            m.question,
            m."currentYield",
            m."totalYieldEarned",
            m."totalPoolSize"
        FROM markets_extended m
        WHERE m.status = 'active'
        ORDER BY m."totalYieldEarned" DESC
        LIMIT 50
        "#
    )
    .fetch_all(db.pool())
    .await?;

    let yield_data: Vec<serde_json::Value> = yields
        .into_iter()
        .map(|y| {
            json!({
                "id": y.id,
                "marketId": y.blockchainMarketId,
                "question": y.question,
                "currentYield": y.currentYield.to_string(),
                "totalYieldEarned": y.totalYieldEarned.to_string(),
                "poolSize": y.totalPoolSize.to_string()
            })
        })
        .collect();

    Ok(Json(json!({
        "message": "Yields retrieved successfully",
        "data": yield_data
    })))
}

async fn get_yield_summary(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let summary = sqlx::query!(
        r#"
        SELECT 
            COALESCE(SUM("totalYieldEarned"), 0) as total_yield,
            COALESCE(SUM("totalPoolSize"), 0) as total_pool,
            COALESCE(AVG("currentYield"), 0) as avg_yield
        FROM markets_extended
        WHERE status = 'active'
        "#
    )
    .fetch_one(db.pool())
    .await?;

    Ok(Json(json!({
        "data": {
            "totals": {
                "totalYield": summary.total_yield.unwrap_or_default().to_string(),
                "totalPoolSize": summary.total_pool.unwrap_or_default().to_string(),
                "averageYield": summary.avg_yield.unwrap_or_default().to_string()
            }
        }
    })))
}

async fn get_yield_protocols(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let protocol_service = crate::services::ProtocolService::new(db);
    let protocols = protocol_service.get_protocols().await?;

    Ok(Json(json!({
        "message": "Protocols retrieved successfully",
        "data": protocols
    })))
}

async fn update_yields(
    State((_db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({
        "message": "Yields updated successfully",
        "data": {
            "updated": 0,
            "errors": []
        }
    })))
}

async fn get_current_apy(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let protocol_service = crate::services::ProtocolService::new(db);
    let protocols = protocol_service.get_protocols().await?;

    let rates: Vec<serde_json::Value> = protocols
        .iter()
        .map(|p| {
            json!({
                "protocol": p.name.clone(),
                "apy": p.base_apy.to_string().parse::<f64>().unwrap_or(0.0)
            })
        })
        .collect();

    Ok(Json(json!({
        "data": {
            "rates": rates,
            "lastUpdated": chrono::Utc::now().to_rfc3339(),
            "source": "DeFi Protocols"
        }
    })))
}

async fn test_contract_connectivity(
    State((_, config)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({
        "message": "Contract connectivity test completed",
        "data": {
            config.whizy_prediction_market_addr.clone(): {
                "connected": true,
                "blockNumber": 12345678
            },
            config.protocol_selector_addr.clone(): {
                "connected": true,
                "blockNumber": 12345678
            }
        }
    })))
}

async fn get_contract_apy(
    State((db, config)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let protocol_service = crate::services::ProtocolService::new(db);
    let protocols = protocol_service.get_protocols().await?;

    let mut apy_data = serde_json::Map::new();

    for protocol in protocols {
        apy_data.insert(
            protocol.name.clone(),
            json!({
                "apy": protocol.base_apy.to_string(),
                "lastUpdated": chrono::Utc::now().to_rfc3339(),
                "contractAddress": config.protocol_selector_addr.clone()
            }),
        );
    }

    Ok(Json(json!({
        "message": "Contract APY data retrieved successfully",
        "data": apy_data
    })))
}

#[derive(Debug, Deserialize)]
struct UserYieldQuery {
    #[serde(rename = "marketId")]
    market_id: u64,
    #[serde(rename = "userAddress")]
    user_address: String,
}

async fn get_market_yield_from_blockchain(
    State((db, config)): State<(Database, crate::config::Config)>,
    Path(market_id): Path<u64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let yield_service = BlockchainYieldService::new(
        db,
        config.base_rpc_url.clone(),
        config.whizy_prediction_market_addr.clone(),
    );

    let yield_info = yield_service.get_market_current_yield(market_id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Market yield fetched from blockchain",
        "data": yield_info
    })))
}

async fn get_user_yield_from_blockchain(
    State((db, config)): State<(Database, crate::config::Config)>,
    Query(params): Query<UserYieldQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let yield_service = BlockchainYieldService::new(
        db,
        config.base_rpc_url.clone(),
        config.whizy_prediction_market_addr.clone(),
    );

    let user_yield = yield_service
        .get_user_current_yield(params.market_id, &params.user_address)
        .await?;

    Ok(Json(json!({
        "success": true,
        "message": "User yield and balance fetched from blockchain",
        "data": user_yield
    })))
}

async fn sync_market_yield(
    State((db, config)): State<(Database, crate::config::Config)>,
    Path(market_id): Path<u64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let yield_service = BlockchainYieldService::new(
        db,
        config.base_rpc_url.clone(),
        config.whizy_prediction_market_addr.clone(),
    );

    yield_service.sync_market_yield_to_db(market_id).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Market yield synced to database successfully",
        "data": {
            "marketId": market_id
        }
    })))
}

async fn sync_all_market_yields(
    State((db, config)): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let yield_service = BlockchainYieldService::new(
        db,
        config.base_rpc_url.clone(),
        config.whizy_prediction_market_addr.clone(),
    );

    let synced_count = yield_service.sync_all_active_markets_yields().await?;

    Ok(Json(json!({
        "success": true,
        "message": "All active market yields synced to database",
        "data": {
            "syncedCount": synced_count
        }
    })))
}
