use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{db::Database, error::AppError, models::*, services::BetService};

pub fn create_bets_router() -> Router<(Database, crate::config::Config)> {
    Router::new()
        .route("/", get(get_bets).post(place_bet))
        .route("/stats/summary", get(get_bet_stats))
        .route("/:id", get(get_bet_by_id))
        .route("/user/:address", get(get_user_bets))
        .route("/market/:market_id", get(get_market_bets))
}

async fn get_bets(
    State((db, _)): State<(Database, crate::config::Config)>,
    Query(params): Query<BetQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let bet_service = BetService::new(db);
    let response = bet_service.get_bets(params).await?;

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

async fn get_bet_by_id(
    State((db, _)): State<(Database, crate::config::Config)>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let bet_service = BetService::new(db);
    let bet = bet_service.get_bet_by_id(&id).await?;
    Ok(Json(json!({
        "data": bet
    })))
}

async fn get_user_bets(
    State((db, _)): State<(Database, crate::config::Config)>,
    Path(address): Path<String>,
    Query(params): Query<BetQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let bet_service = BetService::new(db);
    let response = bet_service.get_bets_by_user(&address, params).await?;

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

async fn get_market_bets(
    State((db, _)): State<(Database, crate::config::Config)>,
    Path(market_id): Path<String>,
    Query(params): Query<BetQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let bet_service = BetService::new(db);
    let response = bet_service.get_bets_by_market(&market_id, params).await?;

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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceBetRequest {
    pub market_identifier: String,
    pub position: bool,
    pub amount: String,
    pub user_address: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BetStatsQueryParams {
    pub user_address: Option<String>,
}

async fn place_bet(
    State((db, _config)): State<(Database, crate::config::Config)>,
    Json(payload): Json<PlaceBetRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let betting_service = crate::services::BettingService::new(db);

    let params = crate::services::betting_service::PlaceBetParams {
        market_identifier: payload.market_identifier,
        user_address: payload.user_address,
        position: payload.position,
        amount: payload.amount,
    };

    let result = betting_service.place_bet(params).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Bet placed successfully",
        "data": {
            "betId": result.bet_id,
            "blockchainBetId": result.blockchain_bet_id,
            "marketId": result.market_id,
            "blockchainMarketId": result.blockchain_market_id,
            "position": result.position,
            "amount": result.amount,
            "txHash": result.tx_hash,
            "userAddress": result.user_address,
            "status": "active"
        }
    })))
}

async fn get_bet_stats(
    State((db, _)): State<(Database, crate::config::Config)>,
    Query(params): Query<BetStatsQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    use sqlx::Row;

    let stats = if let Some(user_address) = params.user_address {
        sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_bets,
                COUNT(*) FILTER (WHERE be.status = 'active') as active_bets,
                COUNT(*) FILTER (WHERE be.status = 'won') as won_bets,
                COUNT(*) FILTER (WHERE be.status = 'lost') as lost_bets,
                COALESCE(SUM(be.amount), 0) as total_amount,
                COALESCE(SUM(CASE WHEN be.status = 'won' THEN be.payout ELSE 0 END), 0) as total_payout,
                COALESCE(SUM(CASE WHEN be.status IN ('won', 'lost') THEN be.amount ELSE 0 END), 0) as resolved_amount
            FROM bets_extended be
            JOIN users u ON be."userId" = u.id
            WHERE u.address = $1
            "#
        )
        .bind(user_address)
        .fetch_one(db.pool())
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_bets,
                COUNT(*) FILTER (WHERE status = 'active') as active_bets,
                COUNT(*) FILTER (WHERE status = 'won') as won_bets,
                COUNT(*) FILTER (WHERE status = 'lost') as lost_bets,
                COALESCE(SUM(amount), 0) as total_amount,
                COALESCE(SUM(CASE WHEN status = 'won' THEN payout ELSE 0 END), 0) as total_payout,
                COALESCE(SUM(CASE WHEN status IN ('won', 'lost') THEN amount ELSE 0 END), 0) as resolved_amount
            FROM bets_extended
            "#
        )
        .fetch_one(db.pool())
        .await?
    };

    let total_amount_val = stats
        .try_get::<bigdecimal::BigDecimal, _>("total_amount")
        .unwrap_or_default();
    let total_payout_val = stats
        .try_get::<bigdecimal::BigDecimal, _>("total_payout")
        .unwrap_or_default();
    let resolved_amount_val = stats
        .try_get::<bigdecimal::BigDecimal, _>("resolved_amount")
        .unwrap_or_default();
    let total_amount = total_amount_val.to_string();
    let total_payout = total_payout_val.to_string();

    let profit = (total_payout_val - resolved_amount_val).to_string();
    let total: i64 = stats.try_get("total_bets").unwrap_or(0);
    let won: i64 = stats.try_get("won_bets").unwrap_or(0);
    let active_bets: i64 = stats.try_get("active_bets").unwrap_or(0);
    let lost_bets: i64 = stats.try_get("lost_bets").unwrap_or(0);
    let win_rate = if total > 0 {
        won as f64 / total as f64
    } else {
        0.0
    };

    Ok(Json(json!({
        "data": {
            "totalBets": total,
            "activeBets": active_bets,
            "wonBets": won,
            "lostBets": lost_bets,
            "winRate": win_rate,
            "totalAmount": total_amount,
            "totalPayout": total_payout,
            "profit": profit
        }
    })))
}
