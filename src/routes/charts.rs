use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};

use crate::{chart::ChartService, db::Database, error::AppError, models::ChartQueryParams};

pub fn create_charts_router() -> Router<(Database, crate::config::Config)> {
    Router::new()
        .route("/market/:id", get(get_market_chart))
        .route("/platform", get(get_platform_chart))
}

async fn get_market_chart(
    State((db, config)): State<(Database, crate::config::Config)>,
    Path(id): Path<String>,
    Query(params): Query<ChartQueryParams>,
) -> Result<Json<Value>, AppError> {
    let chart_service = ChartService::new(db.pool().clone(), config.database_timezone.clone());

    let chart_data = chart_service
        .get_market_chart_data(&id, &params.interval, params.from, params.to)
        .await?;

    let requested_series: Vec<&str> = params.series.split(',').map(|s| s.trim()).collect();

    let mut response_data = json!({});

    if requested_series.contains(&"probability") {
        response_data["probability"] = json!({
            "yes": chart_data.yes_probability,
            "no": chart_data.no_probability
        });
    }

    if requested_series.contains(&"volume") {
        response_data["volume"] = json!({
            "yes": chart_data.yes_volume,
            "no": chart_data.no_volume,
            "total": chart_data.total_volume
        });
    }

    if requested_series.contains(&"odds") {
        response_data["odds"] = json!({
            "yes": chart_data.yes_odds,
            "no": chart_data.no_odds
        });
    }

    if requested_series.contains(&"bets") {
        response_data["bets"] = serde_json::to_value(&chart_data.bet_count).unwrap();
    }

    Ok(Json(json!({
        "success": true,
        "meta": {
            "symbol": id,
            "interval": params.interval,
            "from": params.from,
            "to": params.to,
            "series": requested_series
        },
        "data": response_data
    })))
}

async fn get_platform_chart(
    State((db, config)): State<(Database, crate::config::Config)>,
    Query(_params): Query<ChartQueryParams>,
) -> Result<Json<Value>, AppError> {
    let chart_service = ChartService::new(db.pool().clone(), config.database_timezone.clone());

    let days = 30;
    let chart_data = chart_service.get_platform_chart_data(days).await?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "totalVolume": chart_data.total_volume,
            "activeMarkets": chart_data.active_markets,
            "totalUsers": chart_data.total_users,
            "totalBets": chart_data.total_bets
        },
        "meta": {
            "days": days
        }
    })))
}
