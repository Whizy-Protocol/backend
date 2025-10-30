use bigdecimal::BigDecimal;
use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;

use crate::{
    db::Database,
    error::{AppError, Result},
};

abigen!(
    IWhizyPredictionMarket,
    r#"[
        {
            "type": "function",
            "name": "markets",
            "inputs": [{"name": "marketId", "type": "uint256"}],
            "outputs": [
                {"name": "id", "type": "uint256"},
                {"name": "question", "type": "string"},
                {"name": "endTime", "type": "uint256"},
                {"name": "token", "type": "address"},
                {"name": "vault", "type": "address"},
                {"name": "totalYesShares", "type": "uint256"},
                {"name": "totalNoShares", "type": "uint256"},
                {"name": "resolved", "type": "bool"},
                {"name": "outcome", "type": "bool"},
                {"name": "status", "type": "uint8"}
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getMarketInfo",
            "inputs": [{"name": "marketId", "type": "uint256"}],
            "outputs": [
                {
                    "name": "market",
                    "type": "tuple",
                    "components": [
                        {"name": "id", "type": "uint256"},
                        {"name": "question", "type": "string"},
                        {"name": "endTime", "type": "uint256"},
                        {"name": "token", "type": "address"},
                        {"name": "vault", "type": "address"},
                        {"name": "totalYesShares", "type": "uint256"},
                        {"name": "totalNoShares", "type": "uint256"},
                        {"name": "resolved", "type": "bool"},
                        {"name": "outcome", "type": "bool"},
                        {"name": "status", "type": "uint8"}
                    ]
                },
                {"name": "totalAssets", "type": "uint256"},
                {"name": "currentYield", "type": "uint256"},
                {"name": "yieldWithdrawn", "type": "uint256"}
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "positions",
            "inputs": [
                {"name": "marketId", "type": "uint256"},
                {"name": "user", "type": "address"}
            ],
            "outputs": [
                {"name": "yesShares", "type": "uint256"},
                {"name": "noShares", "type": "uint256"},
                {"name": "claimed", "type": "bool"}
            ],
            "stateMutability": "view"
        },
        {
            "type": "function",
            "name": "getPotentialPayout",
            "inputs": [
                {"name": "marketId", "type": "uint256"},
                {"name": "user", "type": "address"}
            ],
            "outputs": [
                {"name": "yesPayoutIfWin", "type": "uint256"},
                {"name": "noPayoutIfWin", "type": "uint256"},
                {"name": "currentYield", "type": "uint256"}
            ],
            "stateMutability": "view"
        }
    ]"#,
);

abigen!(
    IProtocolSelector,
    r#"[
        function getTotalBalance(address user, address token) external view returns (uint256)
        function getUserDeposit(address user, address token) external view returns (uint256)
    ]"#,
);

abigen!(
    IToken,
    r#"[
        function balanceOf(address account) external view returns (uint256)
    ]"#,
);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketYieldInfo {
    pub blockchain_market_id: u64,
    pub total_pool_size: String,
    pub current_yield_earned: String,
    pub protocol_id: u64,
    pub token_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserYieldInfo {
    pub blockchain_market_id: u64,
    pub user_address: String,
    pub user_bet_amount: String,
    pub user_position: bool,
    pub potential_yield: String,
    pub current_balance: String,
}

pub struct BlockchainYieldService {
    db: Database,
    rpc_url: String,
    whizy_market_address: String,
}

impl BlockchainYieldService {
    pub fn new(db: Database, rpc_url: String, whizy_market_address: String) -> Self {
        Self {
            db,
            rpc_url,
            whizy_market_address,
        }
    }

    pub async fn get_market_current_yield(
        &self,
        blockchain_market_id: u64,
    ) -> Result<MarketYieldInfo> {
        let provider = Provider::<Http>::try_from(&self.rpc_url)
            .map_err(|e| AppError::Internal(format!("Failed to connect to RPC: {}", e)))?;

        let provider = Arc::new(provider);

        let contract_address: Address = self
            .whizy_market_address
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid contract address: {}", e)))?;

        let contract = IWhizyPredictionMarket::new(contract_address, provider.clone());

        let market_info = contract
            .get_market_info(U256::from(blockchain_market_id))
            .call()
            .await
            .map_err(|e| {
                tracing::warn!(
                    "Failed to fetch market {} from blockchain: {}. This may be normal if the market doesn't exist on-chain yet.",
                    blockchain_market_id,
                    e
                );
                AppError::NotFound(format!(
                    "Market {} not found on blockchain. It may not have been created on-chain yet.",
                    blockchain_market_id
                ))
            })?;

        let market = market_info.0;
        let total_assets = market_info.1;
        let current_yield = market_info.2;
        let yield_withdrawn = market_info.3;

        if market.0 != U256::from(blockchain_market_id) {
            return Err(AppError::NotFound(format!(
                "Market {} not found on blockchain",
                blockchain_market_id
            )));
        }

        let token_address = market.3;
        let vault_address = market.4;
        let _total_yes_shares = market.5;
        let _total_no_shares = market.6;

        tracing::info!(
            "Market {} data: totalAssets={}, currentYield={}, yieldWithdrawn={}, vault={:?}",
            blockchain_market_id,
            total_assets,
            current_yield,
            yield_withdrawn,
            vault_address
        );

        let current_yield_earned = current_yield;

        Ok(MarketYieldInfo {
            blockchain_market_id,
            total_pool_size: total_assets.to_string(),
            current_yield_earned: current_yield_earned.to_string(),
            protocol_id: 0,
            token_address: format!("{:?}", token_address),
        })
    }

    pub async fn get_user_current_yield(
        &self,
        blockchain_market_id: u64,
        user_address: &str,
    ) -> Result<UserYieldInfo> {
        let provider = Provider::<Http>::try_from(&self.rpc_url)
            .map_err(|e| AppError::Internal(format!("Failed to connect to RPC: {}", e)))?;

        let provider = Arc::new(provider);

        let user_addr: Address = user_address
            .parse()
            .map_err(|e| AppError::BadRequest(format!("Invalid user address: {}", e)))?;

        let contract_address: Address = self
            .whizy_market_address
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid contract address: {}", e)))?;

        let contract = IWhizyPredictionMarket::new(contract_address, provider.clone());

        let position = contract
            .positions(U256::from(blockchain_market_id), user_addr)
            .call()
            .await
            .map_err(|e| {
                tracing::warn!(
                    "Failed to fetch position for user {} in market {}: {}",
                    user_address,
                    blockchain_market_id,
                    e
                );
                AppError::Internal(format!("Failed to fetch position: {}", e))
            })?;

        let yes_shares = position.0;
        let no_shares = position.1;
        let _claimed = position.2;

        if yes_shares == U256::zero() && no_shares == U256::zero() {
            return Err(AppError::NotFound(format!(
                "No position found for this user in market {}",
                blockchain_market_id
            )));
        }

        let user_position = yes_shares > no_shares;
        let user_total_shares = yes_shares + no_shares;

        let payout_info = contract
            .get_potential_payout(U256::from(blockchain_market_id), user_addr)
            .call()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to fetch potential payout: {}", e)))?;

        let _yes_payout_if_win = payout_info.0;
        let _no_payout_if_win = payout_info.1;
        let current_yield_for_user = payout_info.2;

        let market_info = contract
            .get_market_info(U256::from(blockchain_market_id))
            .call()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to fetch market: {}", e)))?;

        let market = market_info.0;
        let token_address = market.3;

        let token_contract = IToken::new(token_address, provider);
        let balance = token_contract
            .balance_of(user_addr)
            .call()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to fetch user balance: {}", e)))?;

        Ok(UserYieldInfo {
            blockchain_market_id,
            user_address: user_address.to_string(),
            user_bet_amount: user_total_shares.to_string(),
            user_position,
            potential_yield: current_yield_for_user.to_string(),
            current_balance: balance.to_string(),
        })
    }

    pub async fn sync_market_yield_to_db(&self, blockchain_market_id: u64) -> Result<()> {
        let yield_info = self.get_market_current_yield(blockchain_market_id).await?;

        let current_yield = BigDecimal::from_str(&yield_info.current_yield_earned)
            .unwrap_or_else(|_| BigDecimal::from(0));

        sqlx::query!(
            r#"
            UPDATE markets_extended
            SET "currentYield" = $1, "updatedAt" = CURRENT_TIMESTAMP
            WHERE "blockchainMarketId" = $2
            "#,
            current_yield,
            blockchain_market_id as i64
        )
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    pub async fn sync_all_active_markets_yields(&self) -> Result<usize> {
        let markets = sqlx::query!(
            r#"
            SELECT "blockchainMarketId"
            FROM markets_extended
            WHERE status = 'active' AND "blockchainMarketId" IS NOT NULL
            "#
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut synced_count = 0;

        for market in markets {
            if let Some(blockchain_id) = market.blockchainMarketId {
                match self.sync_market_yield_to_db(blockchain_id as u64).await {
                    Ok(_) => synced_count += 1,
                    Err(e) => {
                        tracing::warn!("Failed to sync yield for market {}: {}", blockchain_id, e);
                    }
                }
            }
        }

        Ok(synced_count)
    }
}
