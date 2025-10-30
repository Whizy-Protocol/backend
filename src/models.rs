use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{types::BigDecimal, FromRow};
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum MarketStatus {
    Active,
    Resolved,
    #[default]
    All,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MarketSortBy {
    #[default]
    EndTime,
    TransactionVersion,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

#[derive(Debug, Deserialize)]
pub struct MarketQueryParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub status: MarketStatus,
    #[serde(default)]
    pub sort_by: MarketSortBy,
    #[serde(default)]
    pub order: SortOrder,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
pub struct BetQueryParams {
    #[serde(default = "default_bet_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_bet_limit() -> i64 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketCreated {
    pub id: String,
    pub market_id: String,
    pub question: String,
    pub end_time: String,
    pub token_address: String,
    pub vault_address: Option<String>,
    pub block_number: String,
    pub block_timestamp: String,
    pub transaction_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BetPlaced {
    pub id: String,
    pub market_id: String,
    pub user: String,
    pub position: bool,
    pub amount: String,
    pub shares: Option<String>,
    pub block_number: String,
    pub block_timestamp: String,
    pub transaction_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MarketResolved {
    pub id: String,
    pub market_id: String,
    pub outcome: bool,
    pub block_number: String,
    pub block_timestamp: String,
    pub transaction_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProtocolRegistered {
    pub id: String,
    pub protocol_type: i32,
    pub protocol_address: String,
    pub name: String,
    pub risk_level: i32,
    pub block_number: String,
    pub block_timestamp: String,
    pub transaction_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarketExtended {
    pub id: String,
    #[serde(rename = "blockchainMarketId")]
    #[sqlx(rename = "blockchainMarketId")]
    pub blockchain_market_id: Option<i64>,
    #[serde(rename = "marketId")]
    #[sqlx(rename = "marketId")]
    pub market_id: Option<String>,
    #[serde(rename = "adjTicker")]
    #[sqlx(rename = "adjTicker")]
    pub adj_ticker: Option<String>,
    pub platform: String,
    pub question: Option<String>,
    pub description: Option<String>,
    pub rules: Option<String>,
    pub status: String,
    pub probability: i32,
    pub volume: BigDecimal,
    #[serde(rename = "openInterest")]
    #[sqlx(rename = "openInterest")]
    pub open_interest: BigDecimal,
    #[serde(rename = "endDate")]
    #[sqlx(rename = "endDate")]
    pub end_date: NaiveDateTime,
    #[serde(rename = "resolutionDate")]
    #[sqlx(rename = "resolutionDate")]
    pub resolution_date: Option<NaiveDateTime>,
    pub result: Option<bool>,
    pub link: Option<String>,
    #[serde(rename = "imageUrl")]
    #[sqlx(rename = "imageUrl")]
    pub image_url: Option<String>,
    #[serde(rename = "totalPoolSize")]
    #[sqlx(rename = "totalPoolSize")]
    pub total_pool_size: BigDecimal,
    #[serde(rename = "yesPoolSize")]
    #[sqlx(rename = "yesPoolSize")]
    pub yes_pool_size: BigDecimal,
    #[serde(rename = "noPoolSize")]
    #[sqlx(rename = "noPoolSize")]
    pub no_pool_size: BigDecimal,
    #[serde(rename = "countYes")]
    #[sqlx(rename = "countYes")]
    pub count_yes: i32,
    #[serde(rename = "countNo")]
    #[sqlx(rename = "countNo")]
    pub count_no: i32,
    #[serde(rename = "currentYield")]
    #[sqlx(rename = "currentYield")]
    pub current_yield: BigDecimal,
    #[serde(rename = "totalYieldEarned")]
    #[sqlx(rename = "totalYieldEarned")]
    pub total_yield_earned: BigDecimal,
    #[serde(rename = "totalYieldUntilEnd")]
    #[sqlx(skip)]
    pub total_yield_until_end: Option<BigDecimal>,
    #[serde(rename = "createdAt")]
    #[sqlx(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    #[sqlx(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

impl MarketExtended {
    pub fn calculate_total_yield_until_end(&mut self, best_apy: f64) {
        let now = chrono::Utc::now().naive_utc();
        let end_date = self.end_date;

        if end_date > now && self.total_pool_size > BigDecimal::from(0) {
            let duration = end_date.signed_duration_since(now);
            let days_remaining = duration.num_days().max(0) as f64;

            let pool_size_f64: f64 = self.total_pool_size.to_string().parse().unwrap_or(0.0);
            let apy_decimal = best_apy / 100.0;
            let projected_yield = pool_size_f64 * apy_decimal * (days_remaining / 365.0);

            let earned_f64: f64 = self.total_yield_earned.to_string().parse().unwrap_or(0.0);
            let total_until_end = earned_f64 + projected_yield;

            self.total_yield_until_end = BigDecimal::from_str(&total_until_end.to_string()).ok();
        } else {
            self.total_yield_until_end = Some(self.total_yield_earned.clone());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub address: String,
    pub username: Option<String>,
    #[serde(rename = "avatarUrl")]
    #[sqlx(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
    #[serde(rename = "createdAt")]
    #[sqlx(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    #[sqlx(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BetExtended {
    pub id: String,
    #[serde(rename = "blockchainBetId")]
    #[sqlx(rename = "blockchainBetId")]
    pub blockchain_bet_id: Option<i64>,
    #[serde(rename = "userId")]
    #[sqlx(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "marketId")]
    #[sqlx(rename = "marketId")]
    pub market_id: Option<String>,
    pub position: Option<bool>,
    pub amount: Option<BigDecimal>,
    pub shares: Option<BigDecimal>,
    pub odds: BigDecimal,
    pub status: String,
    pub payout: Option<BigDecimal>,
    #[serde(rename = "createdAt")]
    #[sqlx(rename = "createdAt")]
    pub created_at: NaiveDateTime,
    #[serde(rename = "updatedAt")]
    #[sqlx(rename = "updatedAt")]
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Protocol {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing, rename = "protocolType")]
    #[sqlx(rename = "protocolType")]
    pub protocol_type: Option<i32>,
    #[serde(skip_serializing)]
    pub address: Option<String>,
    #[serde(rename = "baseApy")]
    #[sqlx(rename = "baseApy")]
    pub base_apy: BigDecimal,
    #[serde(skip_serializing)]
    pub tvl: Option<BigDecimal>,
    #[serde(skip_serializing, rename = "riskLevel")]
    #[sqlx(rename = "riskLevel")]
    pub risk_level: Option<i32>,
    #[serde(rename = "isActive")]
    #[sqlx(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "iconUrl")]
    #[sqlx(rename = "iconUrl")]
    pub icon_url: Option<String>,
    #[serde(rename = "createdAt")]
    #[sqlx(rename = "createdAt")]
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct YieldRecord {
    pub id: String,
    #[serde(rename = "marketId")]
    #[sqlx(rename = "marketId")]
    pub market_id: String,
    #[serde(rename = "protocolId")]
    #[sqlx(rename = "protocolId")]
    pub protocol_id: String,
    pub amount: BigDecimal,
    pub apy: BigDecimal,
    #[serde(rename = "yieldAmount")]
    pub yield_amount: BigDecimal,
    pub period: NaiveDateTime,
    #[serde(rename = "createdAt")]
    #[sqlx(rename = "createdAt")]
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct FeeRecord {
    pub id: String,
    #[serde(rename = "marketId")]
    #[sqlx(rename = "marketId")]
    pub market_id: Option<String>,
    #[serde(rename = "feeType")]
    #[sqlx(rename = "feeType")]
    pub fee_type: String,
    pub amount: BigDecimal,
    pub source: String,
    #[serde(rename = "createdAt")]
    #[sqlx(rename = "createdAt")]
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MarketStats {
    pub market_id: i64,
    pub total_bets: i64,
    pub total_volume: String,
    pub yes_volume: String,
    pub no_volume: String,
    pub yes_percentage: f64,
    pub no_percentage: f64,
    pub unique_bettors: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformStats {
    pub total_markets: i64,
    pub active_markets: i64,
    pub resolved_markets: i64,
    pub total_bets: i64,
    pub total_volume: String,
    pub unique_users: i64,
    pub total_yield_earned: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserStats {
    pub user_addr: String,
    pub total_bets: i64,
    pub total_wagered: String,
    pub markets_participated: i64,
    pub wins: i64,
    pub losses: i64,
    pub pending: i64,
    pub total_winnings: String,
    pub total_yield_earned: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiInfoResponse {
    pub name: String,
    pub version: String,
    pub network: String,
    pub chain_id: u64,
    pub contracts: ContractAddresses,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ContractAddresses {
    pub prediction_market: String,
    pub protocol_selector: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SyncStatusResponse {
    pub last_synced_block: i64,
    pub is_syncing: bool,
    pub markets_synced: i64,
    pub bets_synced: i64,
    pub protocols_synced: i64,
    pub last_sync_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MarketResponse {
    pub data: Vec<MarketExtended>,
    pub meta: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct BetResponse {
    pub data: Vec<BetExtended>,
    pub meta: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WalletConnectRequest {
    pub address: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletConnectResponse {
    pub message: String,
    pub data: WalletConnectData,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletConnectData {
    pub user: User,
    pub token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateProfileResponse {
    pub message: String,
    pub data: UpdateProfileData,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateProfileData {
    pub user: User,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserWithBets {
    #[serde(flatten)]
    pub user: User,
    pub bets: Vec<BetExtended>,
    pub total_bets: i64,
}

#[derive(Debug, Deserialize)]
pub struct ChartQueryParams {
    #[serde(default = "default_chart_interval")]
    pub interval: String,
    pub from: Option<i64>,
    pub to: Option<i64>,
    #[serde(default = "default_chart_series")]
    pub series: String,
}

fn default_chart_interval() -> String {
    "1h".to_string()
}

fn default_chart_series() -> String {
    "probability,volume,odds,bets".to_string()
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMarketRequest {
    pub question: String,
    pub description: Option<String>,
    pub end_time: i64,
    pub token_address: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PlaceBetRequest {
    pub market_id: String,
    pub position: bool,
    pub amount: String,
}
