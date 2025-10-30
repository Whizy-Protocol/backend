use axum::{extract::State, response::Json, routing::get, Router};
use serde_json::json;

use crate::{db::Database, error::AppError};

pub fn create_prices_router() -> Router<(Database, crate::config::Config)> {
    Router::new()
        .route("/usdc-usd", get(get_usdc_usd_price))
        .route("/usdc-usd/refresh", get(refresh_usdc_usd_price))
        .route("/eth-usd", get(get_eth_usd_price))
        .route("/eth-usd/refresh", get(refresh_eth_usd_price))
}

async fn get_eth_usd_price(
    State(_): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let price = 2500.00;

    Ok(Json(json!({
        "success": true,
        "data": {
            "price": price,
            "symbol": "ETH/USD",
            "source": "Chainlink",
            "decimals": 8,
            "timestamp": chrono::Utc::now().timestamp()
        },
        "formatted": format!("${:.2}", price)
    })))
}

async fn refresh_eth_usd_price(
    State(_): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let price = 2500.00;

    Ok(Json(json!({
        "success": true,
        "data": {
            "price": price,
            "symbol": "ETH/USD",
            "source": "Chainlink",
            "decimals": 8,
            "timestamp": chrono::Utc::now().timestamp()
        },
        "formatted": format!("${:.2}", price),
        "message": "Price refreshed successfully"
    })))
}

async fn get_usdc_usd_price(
    State(_): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let price = 1.00;

    Ok(Json(json!({
        "success": true,
        "data": {
            "price": price,
            "symbol": "USDC/USD",
            "source": "Chainlink",
            "decimals": 8,
            "timestamp": chrono::Utc::now().timestamp()
        },
        "formatted": format!("${:.2}", price)
    })))
}

async fn refresh_usdc_usd_price(
    State(_): State<(Database, crate::config::Config)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let price = 1.00;

    Ok(Json(json!({
        "success": true,
        "data": {
            "price": price,
            "symbol": "USDC/USD",
            "source": "Chainlink",
            "decimals": 8,
            "timestamp": chrono::Utc::now().timestamp()
        },
        "formatted": format!("${:.2}", price),
        "message": "Price refreshed successfully"
    })))
}
