use axum::{
    extract::State,
    middleware,
    response::Json,
    routing::{get, post, put},
    Extension, Router,
};
use serde_json::json;

use crate::{
    db::Database,
    error::AppError,
    middleware::jwt::require_jwt,
    models::{
        UpdateProfileData, UpdateProfileRequest, UpdateProfileResponse, WalletConnectData,
        WalletConnectRequest, WalletConnectResponse,
    },
    services::UserService,
    utils::jwt::{Claims, JwtService},
};

pub fn create_auth_router() -> Router<(Database, crate::config::Config)> {
    let public_routes = Router::new().route("/wallet", post(connect_wallet));

    let protected_routes = Router::new()
        .route("/me", get(get_current_user))
        .route("/profile", put(update_profile))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route_layer(middleware::from_fn(require_jwt));

    public_routes.merge(protected_routes)
}

async fn connect_wallet(
    State((db, _)): State<(Database, crate::config::Config)>,
    Json(payload): Json<WalletConnectRequest>,
) -> Result<Json<WalletConnectResponse>, AppError> {
    let user_service = UserService::new(db.clone());
    let jwt_service = JwtService::new();

    if payload.address.is_empty() {
        return Err(AppError::BadRequest("Address cannot be empty".to_string()));
    }

    let user = user_service.upsert_user(&payload.address).await?;

    let token = jwt_service
        .generate_token(user.id.clone(), user.address.clone())
        .map_err(|e| AppError::Internal(format!("Failed to generate token: {}", e)))?;

    Ok(Json(WalletConnectResponse {
        message: "Successfully connected wallet".to_string(),
        data: WalletConnectData { user, token },
    }))
}

async fn get_current_user(
    State((db, _)): State<(Database, crate::config::Config)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_service = UserService::new(db.clone());

    let user = user_service.get_user_by_id(&claims.sub).await?;

    let bets_with_market = sqlx::query(
        r#"
        SELECT 
            b.id,
            b."blockchainBetId",
            b."userId",
            b."marketId",
            b.position,
            b.amount,
            b.odds,
            b.status,
            b.payout,
            b."createdAt",
            b."updatedAt",
            m.question as market_question,
            m."imageUrl" as market_image_url,
            m."endDate" as market_end_date,
            m.status as market_status
        FROM bets_extended b
        LEFT JOIN markets_extended m ON b."marketId" = m.id
        WHERE b."userId" = $1
        ORDER BY b."createdAt" DESC
        "#,
    )
    .bind(&claims.sub)
    .fetch_all(db.pool())
    .await?;

    use sqlx::Row;
    let bets_json: Vec<serde_json::Value> = bets_with_market
        .iter()
        .map(|row| {
            json!({
                "id": row.get::<String, _>("id"),
                "blockchainBetId": row.get::<Option<i64>, _>("blockchainBetId"),
                "userId": row.get::<String, _>("userId"),
                "marketId": row.get::<Option<String>, _>("marketId"),
                "position": row.get::<Option<bool>, _>("position"),
                "amount": row.get::<Option<bigdecimal::BigDecimal>, _>("amount").map(|v| v.to_string()),
                "odds": row.get::<bigdecimal::BigDecimal, _>("odds").to_string(),
                "status": row.get::<String, _>("status"),
                "payout": row.get::<Option<bigdecimal::BigDecimal>, _>("payout").map(|v| v.to_string()),
                "createdAt": row.get::<chrono::NaiveDateTime, _>("createdAt"),
                "updatedAt": row.get::<chrono::NaiveDateTime, _>("updatedAt"),
                "market": {
                    "question": row.get::<Option<String>, _>("market_question"),
                    "imageUrl": row.get::<Option<String>, _>("market_image_url"),
                    "endDate": row.get::<Option<chrono::NaiveDateTime>, _>("market_end_date"),
                    "status": row.get::<Option<String>, _>("market_status")
                }
            })
        })
        .collect();

    Ok(Json(json!({
        "data": {
            "id": user.id,
            "address": user.address,
            "username": user.username,
            "avatarUrl": user.avatar_url,
            "createdAt": user.created_at,
            "updatedAt": user.updated_at,
            "bets": bets_json,
            "totalBets": bets_json.len()
        }
    })))
}

async fn update_profile(
    State((db, _)): State<(Database, crate::config::Config)>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<UpdateProfileResponse>, AppError> {
    let user_service = UserService::new(db);

    let user = user_service
        .update_user_profile(&claims.address, payload.username, payload.avatar_url)
        .await?;

    Ok(Json(UpdateProfileResponse {
        message: "Profile updated successfully".to_string(),
        data: UpdateProfileData { user },
    }))
}

async fn refresh_token(
    State((db, _)): State<(Database, crate::config::Config)>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_service = UserService::new(db);
    let jwt_service = JwtService::new();

    let user = user_service.get_user_by_id(&claims.sub).await?;

    let token = jwt_service
        .generate_token(user.id.clone(), user.address.clone())
        .map_err(|e| AppError::Internal(format!("Failed to generate token: {}", e)))?;

    Ok(Json(json!({
        "message": "Token refreshed successfully",
        "data": {
            "token": token
        }
    })))
}

async fn logout() -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(json!({
        "message": "Successfully logged out"
    })))
}
