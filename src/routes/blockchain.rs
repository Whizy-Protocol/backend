use axum::{extract::State, response::Json, routing::get, Router};
use serde_json::json;

use crate::{config::Config, db::Database, error::AppError};

pub fn create_blockchain_router() -> Router<(Database, Config)> {
    Router::new()
        .route("/info", get(get_blockchain_info))
        .route("/status", get(get_blockchain_info))
        .route("/latest-block", get(get_latest_block))
}

async fn get_blockchain_info(
    State((_, config)): State<(Database, Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({
        "network": "HEDERA Testnet",
        "chainId": config.base_chain_id,
        "rpcUrl": config.base_rpc_url,
        "contracts": {
            "predictionMarket": config.whizy_prediction_market_addr,
            "protocolSelector": config.protocol_selector_addr
        }
    })))
}

async fn get_latest_block(
    State((_, _config)): State<(Database, Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({
        "blockNumber": 12345678,
        "timestamp": chrono::Utc::now().timestamp(),
        "network": "HEDERA Testnet"
    })))
}
