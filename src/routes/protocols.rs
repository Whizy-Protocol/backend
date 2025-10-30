use axum::{
    extract::{Path, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Serialize;

use crate::{db::Database, error::AppError, models::Protocol, services::ProtocolService};

pub fn create_protocols_router() -> Router<(Database, crate::config::Config)> {
    Router::new()
        .route("/", get(get_protocols))
        .route("/:address", get(get_protocol))
        .route("/refresh-apy", post(refresh_protocol_apys))
}

async fn get_protocols(
    State((db, _)): State<(Database, crate::config::Config)>,
) -> Result<Json<Vec<Protocol>>, AppError> {
    let protocol_service = ProtocolService::new(db);
    let protocols = protocol_service.get_protocols().await?;
    Ok(Json(protocols))
}

async fn get_protocol(
    State((db, _)): State<(Database, crate::config::Config)>,
    Path(address): Path<String>,
) -> Result<Json<Protocol>, AppError> {
    let protocol_service = ProtocolService::new(db);
    let protocol = protocol_service.get_protocol_by_address(&address).await?;
    Ok(Json(protocol))
}

#[derive(Serialize)]
struct RefreshApyResponse {
    success: bool,
    updated_count: usize,
    message: String,
}

async fn refresh_protocol_apys(
    State((db, config)): State<(Database, crate::config::Config)>,
) -> Result<Json<RefreshApyResponse>, AppError> {
    let protocol_service = ProtocolService::new(db);

    match protocol_service
        .update_all_apys_from_blockchain(&config.base_rpc_url)
        .await
    {
        Ok(count) => Ok(Json(RefreshApyResponse {
            success: true,
            updated_count: count,
            message: format!(
                "Successfully updated {} protocol APYs from blockchain",
                count
            ),
        })),
        Err(e) => Ok(Json(RefreshApyResponse {
            success: false,
            updated_count: 0,
            message: format!("Failed to update protocol APYs: {}", e),
        })),
    }
}
