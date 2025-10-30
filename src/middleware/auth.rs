use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

pub async fn require_api_key(req: Request, next: Next) -> Result<Response, Response> {
    let api_key_header = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok());

    let expected_key = std::env::var("API_KEY").unwrap_or_else(|_| "dev-api-key".to_string());

    match api_key_header {
        Some(key) if key == expected_key => Ok(next.run(req).await),
        _ => Err((StatusCode::UNAUTHORIZED, "Invalid or missing API key").into_response()),
    }
}
