use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Whizy Prediction Markets API - HEDERA Testnet",
        version = "1.0.0",
        description = "REST API for Whizy Prediction Markets on HEDERA Testnet network",
        contact(
            name = "Whizy Team",
            email = "dev@whizy.io"
        )
    ),
    servers(
        (url = "http://localhost:3002/api", description = "Local development server"),
        (url = "https://api.whizy.io/api", description = "Production server")
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "markets", description = "Prediction market operations"),
        (name = "bets", description = "Betting operations"),
        (name = "users", description = "User management"),
        (name = "protocols", description = "DeFi protocol information"),
        (name = "stats", description = "Statistics and analytics"),
        (name = "charts", description = "Time-series chart data"),
        (name = "yields", description = "Yield calculations"),
        (name = "prices", description = "Price feeds"),
        (name = "sync", description = "Data synchronization"),
        (name = "blockchain", description = "Blockchain information"),
        (name = "health", description = "System health checks")
    ),
    components(schemas(
        crate::models::WalletConnectRequest,
        crate::models::WalletConnectResponse,
        crate::models::UpdateProfileRequest,
        crate::models::UpdateProfileResponse,
        crate::models::User,
        crate::models::MarketExtended,
        crate::models::BetExtended,
        crate::models::Protocol,
        crate::models::MarketStats,
        crate::models::PlatformStats,
        crate::models::UserStats,
        crate::models::HealthResponse,
        crate::models::ApiInfoResponse,
        crate::models::SyncStatusResponse,
    ))
)]
pub struct ApiDoc;
