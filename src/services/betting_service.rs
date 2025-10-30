use crate::{
    constants::{parse_usdc_amount, raw_to_bigdecimal, raw_to_usdc, validate_bet_amount},
    db::Database,
    error::AppError,
};
use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

abigen!(
    WhizyPredictionMarket,
    r#"[
        function placeBet(uint256 marketId, bool isYes, uint256 amount) external
        function markets(uint256) external view returns (uint256 id, string question, uint256 endTime, address token, address vault, uint256 totalYesShares, uint256 totalNoShares, bool resolved, bool outcome, uint8 status)
        event BetPlaced(uint256 indexed marketId, address indexed user, bool position, uint256 amount, uint256 shares)
    ]"#,
);

abigen!(
    IERC20,
    r#"[
        function approve(address spender, uint256 amount) external returns (bool)
        function allowance(address owner, address spender) external view returns (uint256)
        function balanceOf(address account) external view returns (uint256)
    ]"#,
);

#[derive(Debug, Serialize, Deserialize)]
pub struct PlaceBetParams {
    pub market_identifier: String,
    pub user_address: String,
    pub position: bool,
    pub amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlaceBetResult {
    pub bet_id: String,
    pub blockchain_bet_id: u64,
    pub market_id: String,
    pub blockchain_market_id: u64,
    pub position: bool,
    pub amount: String,
    pub tx_hash: String,
    pub user_address: String,
}

pub struct BettingService {
    db: Database,
}

impl BettingService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn place_bet(&self, params: PlaceBetParams) -> Result<PlaceBetResult, AppError> {
        info!("Placing bet on market: {}", params.market_identifier);

        let market = self
            .get_market_by_identifier(&params.market_identifier)
            .await?;

        if market.blockchain_market_id.is_none() {
            return Err(AppError::BadRequest(
                "Market is not on blockchain yet".to_string(),
            ));
        }

        let blockchain_market_id = market.blockchain_market_id.unwrap() as u64;

        if market.status != "active" {
            return Err(AppError::BadRequest("Market is not active".to_string()));
        }

        let amount_raw = parse_usdc_amount(&params.amount)?;

        validate_bet_amount(amount_raw)?;

        info!(
            "Placing bet: {} USDC ({} base units)",
            raw_to_usdc(amount_raw),
            amount_raw
        );

        let tx_hash = self
            .submit_bet_transaction(blockchain_market_id, params.position, amount_raw)
            .await?;

        info!("Bet transaction submitted: {}", tx_hash);

        let odds = self
            .calculate_bet_odds(&market.id, params.position, amount_raw)
            .await?;
        let odds_decimal = format!("{:.4}", odds)
            .parse::<sqlx::types::BigDecimal>()
            .unwrap_or_else(|_| "1.0".parse::<sqlx::types::BigDecimal>().unwrap());

        let bet_id = Uuid::new_v4().to_string();

        sqlx::query!(
            r#"
            INSERT INTO bets_extended (
                id, "userId", "marketId", position, amount,
                odds, status, "createdAt", "updatedAt"
            )
            VALUES ($1, $2, $3, $4, $5, $6, 'active', NOW(), NOW())
            "#,
            bet_id,
            params.user_address,
            market.id,
            params.position,
            raw_to_bigdecimal(amount_raw),
            odds_decimal,
        )
        .execute(self.db.pool())
        .await
        .map_err(|e| AppError::Internal(format!("Failed to insert bet: {}", e)))?;

        self.update_market_pools(&market.id, params.position, amount_raw)
            .await?;

        info!(
            "Bet {} created successfully for market {}",
            bet_id, market.id
        );

        Ok(PlaceBetResult {
            bet_id,
            blockchain_bet_id: 0,
            market_id: market.id,
            blockchain_market_id,
            position: params.position,
            amount: params.amount,
            tx_hash,
            user_address: params.user_address,
        })
    }

    async fn get_market_by_identifier(&self, identifier: &str) -> Result<MarketRecord, AppError> {
        let market = sqlx::query_as!(
            MarketRecord,
            r#"
            SELECT id, "blockchainMarketId" as blockchain_market_id, status
            FROM markets_extended
            WHERE id = $1 OR "adjTicker" = $1 OR "blockchainMarketId"::text = $1
            LIMIT 1
            "#,
            identifier
        )
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?
        .ok_or_else(|| AppError::NotFound("Market not found".to_string()))?;

        Ok(market)
    }

    async fn submit_bet_transaction(
        &self,
        market_id: u64,
        position: bool,
        amount: u64,
    ) -> Result<String, AppError> {
        let rpc_url = std::env::var("HEDERA_RPC_URL")
            .map_err(|_| AppError::Internal("HEDERA_RPC_URL not configured".to_string()))?;

        let contract_address = std::env::var("BASE_CONTRACT_ADDRESS")
            .map_err(|_| AppError::Internal("BASE_CONTRACT_ADDRESS not configured".to_string()))?;

        let private_key = std::env::var("USER_PRIVATE_KEY")
            .map_err(|_| AppError::Internal("USER_PRIVATE_KEY not configured".to_string()))?;

        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| AppError::Internal(format!("Failed to connect to RPC: {}", e)))?;

        // Read chain ID from environment
        let chain_id = std::env::var("HEDERA_CHAIN_ID")
            .unwrap_or_else(|_| "2484".to_string())
            .parse::<u64>()
            .unwrap_or(2484);

        let wallet: LocalWallet = private_key
            .parse::<LocalWallet>()
            .map_err(|e| AppError::Internal(format!("Invalid private key: {}", e)))?
            .with_chain_id(chain_id);

        let client = SignerMiddleware::new(provider, wallet);
        let client = Arc::new(client);

        let address: Address = contract_address
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid contract address: {}", e)))?;

        let contract = WhizyPredictionMarket::new(address, client.clone());

        info!(
            "Submitting bet transaction: market={}, position={}, amount={} ({} USDC)",
            market_id,
            position,
            amount,
            raw_to_usdc(amount)
        );

        let market_data = contract
            .markets(U256::from(market_id))
            .call()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get market data: {}", e)))?;

        let usdc_address = market_data.3;

        let usdc_contract = IERC20::new(usdc_address, client.clone());

        let wallet_address = client.address();
        let allowance = usdc_contract
            .allowance(wallet_address, address)
            .call()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to check allowance: {}", e)))?;

        info!("Current USDC allowance: {} (need: {})", allowance, amount);

        if allowance < U256::from(amount) {
            info!("Insufficient allowance, approving USDC...");

            let approve_call = usdc_contract.approve(address, U256::from(u64::MAX));
            let approve_pending = approve_call
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to send approval: {}", e)))?;

            let tx_hash = approve_pending.tx_hash();
            info!("Approval transaction sent: {:?}", tx_hash);

            approve_pending
                .await
                .map_err(|e| AppError::Internal(format!("Approval transaction failed: {}", e)))?;

            info!("✅ USDC approved successfully");
        } else {
            info!("✅ Sufficient USDC allowance already exists");
        }

        let pending_tx = contract.place_bet(U256::from(market_id), position, U256::from(amount));

        let pending_tx = pending_tx
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send transaction: {}", e)))?;

        let tx_hash = format!("{:?}", pending_tx.tx_hash());
        info!("Transaction sent: {}", tx_hash);

        match pending_tx.await {
            Ok(Some(receipt)) => {
                info!(
                    "Transaction confirmed in block: {}",
                    receipt.block_number.unwrap_or_default()
                );
            }
            Ok(None) => {
                error!("Transaction returned no receipt");
                return Err(AppError::Internal(
                    "Transaction returned no receipt".to_string(),
                ));
            }
            Err(e) => {
                error!("Transaction failed: {}", e);
                return Err(AppError::Internal(format!("Transaction failed: {}", e)));
            }
        }

        Ok(tx_hash)
    }

    async fn calculate_bet_odds(
        &self,
        market_id: &str,
        position: bool,
        amount_raw: u64,
    ) -> Result<f64, AppError> {
        let market = sqlx::query!(
            r#"
            SELECT "yesPoolSize", "noPoolSize", "totalPoolSize"
            FROM markets_extended
            WHERE id = $1
            "#,
            market_id
        )
        .fetch_one(self.db.pool())
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch market: {}", e)))?;

        let amount_usdc = raw_to_usdc(amount_raw);

        let yes_pool_raw: f64 = market.yesPoolSize.to_string().parse::<f64>().unwrap_or(0.0);
        let no_pool_raw: f64 = market.noPoolSize.to_string().parse::<f64>().unwrap_or(0.0);

        let yes_pool = raw_to_usdc(yes_pool_raw as u64);
        let no_pool = raw_to_usdc(no_pool_raw as u64);

        let (final_yes_pool, final_no_pool) = if position {
            (yes_pool + amount_usdc, no_pool)
        } else {
            (yes_pool, no_pool + amount_usdc)
        };

        let total_pool = final_yes_pool + final_no_pool;

        let raw_odds = if position {
            if final_yes_pool > 0.0 {
                total_pool / final_yes_pool
            } else {
                1.0
            }
        } else if final_no_pool > 0.0 {
            total_pool / final_no_pool
        } else {
            1.0
        };

        let odds = raw_odds.max(1.0);

        Ok(odds)
    }

    async fn update_market_pools(
        &self,
        market_id: &str,
        position: bool,
        amount_raw: u64,
    ) -> Result<(), AppError> {
        let amount_decimal = raw_to_bigdecimal(amount_raw);

        if position {
            sqlx::query!(
                r#"
                UPDATE markets_extended
                SET "yesPoolSize" = "yesPoolSize" + $1,
                    "totalPoolSize" = "totalPoolSize" + $1,
                    volume = "yesPoolSize" + "noPoolSize" + $1,
                    "countYes" = "countYes" + 1,
                    "updatedAt" = NOW()
                WHERE id = $2
                "#,
                amount_decimal,
                market_id
            )
            .execute(self.db.pool())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update market pools: {}", e)))?;
        } else {
            sqlx::query!(
                r#"
                UPDATE markets_extended
                SET "noPoolSize" = "noPoolSize" + $1,
                    "totalPoolSize" = "totalPoolSize" + $1,
                    volume = "yesPoolSize" + "noPoolSize" + $1,
                    "countNo" = "countNo" + 1,
                    "updatedAt" = NOW()
                WHERE id = $2
                "#,
                amount_decimal,
                market_id
            )
            .execute(self.db.pool())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to update market pools: {}", e)))?;
        }

        info!(
            "Updated market {} pools: position={}, amount={} ({} USDC)",
            market_id,
            position,
            amount_raw,
            raw_to_usdc(amount_raw)
        );

        Ok(())
    }
}

#[derive(Debug)]
struct MarketRecord {
    id: String,
    blockchain_market_id: Option<i64>,
    status: String,
}
