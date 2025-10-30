use crate::{
    db::Database,
    error::{AppError, Result},
    models::*,
};
use tracing::info;
use uuid::Uuid;

pub struct BetService {
    db: Database,
}

impl BetService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn get_bets(&self, _params: BetQueryParams) -> Result<BetResponse> {
        Ok(BetResponse {
            data: vec![],
            meta: PaginationMeta {
                total: 0,
                limit: 0,
                offset: 0,
                has_more: false,
            },
        })
    }

    pub async fn get_bets_by_market(
        &self,
        _market_id: &str,
        _params: BetQueryParams,
    ) -> Result<BetResponse> {
        Ok(BetResponse {
            data: vec![],
            meta: PaginationMeta {
                total: 0,
                limit: 0,
                offset: 0,
                has_more: false,
            },
        })
    }

    pub async fn get_bets_by_user(
        &self,
        _user_address: &str,
        _params: BetQueryParams,
    ) -> Result<BetResponse> {
        Ok(BetResponse {
            data: vec![],
            meta: PaginationMeta {
                total: 0,
                limit: 0,
                offset: 0,
                has_more: false,
            },
        })
    }

    pub async fn get_bet_by_id(&self, _id: &str) -> Result<BetExtended> {
        Err(crate::error::AppError::NotFound(
            "Not implemented".to_string(),
        ))
    }

    pub async fn upsert_bet_from_indexer(&self, _bet: &BetPlaced) -> Result<String> {
        Ok("".to_string())
    }

    pub async fn get_indexer_bets(&self) -> Result<Vec<BetPlaced>> {
        Ok(vec![])
    }

    pub async fn sync_bets_from_indexer(&self) -> Result<usize> {
        info!("Starting to sync bets from indexer...");

        let unprocessed_bets = sqlx::query!(
            r#"
            SELECT 
                bp.id,
                bp.market_id,
                bp."user",
                bp.position,
                bp.amount,
                bp.shares
            FROM bet_placeds bp
            LEFT JOIN bets_extended be ON be.id = bp.id
            WHERE be.id IS NULL
            ORDER BY bp.block_number ASC
            LIMIT 100
            "#
        )
        .fetch_all(self.db.pool())
        .await?;

        if unprocessed_bets.is_empty() {
            return Ok(0);
        }

        info!(
            "Found {} unprocessed bets from indexer",
            unprocessed_bets.len()
        );

        let mut synced_count = 0;

        for bet_record in unprocessed_bets {
            let event_id = &bet_record.id;
            let market_id_numeric = &bet_record.market_id;
            let user_address = &bet_record.user;
            let position = bet_record.position;
            let amount = &bet_record.amount;
            let shares = &bet_record.shares;

            let market_id = market_id_numeric.to_string().parse::<i64>().unwrap_or(0);

            let user = sqlx::query!(r#"SELECT id FROM users WHERE address = $1"#, user_address)
                .fetch_optional(self.db.pool())
                .await?;

            let user_uuid = if let Some(existing_user) = user {
                existing_user.id
            } else {
                let new_user_id = Uuid::new_v4().to_string();
                sqlx::query!(
                    r#"INSERT INTO users (id, address, "createdAt", "updatedAt") VALUES ($1, $2, NOW(), NOW())"#,
                    new_user_id,
                    user_address
                )
                .execute(self.db.pool())
                .await
                .map_err(|e| AppError::Internal(format!("Failed to create user: {}", e)))?;

                info!(
                    "Created new user {} for address {}",
                    new_user_id, user_address
                );
                new_user_id
            };

            let market = sqlx::query!(
                r#"SELECT id, "yesPoolSize", "noPoolSize", "totalPoolSize" FROM markets_extended WHERE "blockchainMarketId" = $1"#,
                market_id
            )
            .fetch_optional(self.db.pool())
            .await?;

            if market.is_none() {
                info!("Market {} not found in database, skipping bet", market_id);
                continue;
            }

            let market_data = market.unwrap();
            let market_uuid = market_data.id;

            let yes_pool = market_data
                .yesPoolSize
                .to_string()
                .parse::<f64>()
                .unwrap_or(1.0);
            let no_pool = market_data
                .noPoolSize
                .to_string()
                .parse::<f64>()
                .unwrap_or(1.0);
            let total_pool = market_data
                .totalPoolSize
                .to_string()
                .parse::<f64>()
                .unwrap_or(2.0);

            let calculated_odds = if position {
                if yes_pool > 0.0 {
                    total_pool / yes_pool
                } else {
                    1.0
                }
            } else if no_pool > 0.0 {
                total_pool / no_pool
            } else {
                1.0
            };

            use bigdecimal::BigDecimal;
            use std::str::FromStr;
            let odds_decimal = BigDecimal::from_str(&format!("{:.2}", calculated_odds))
                .unwrap_or_else(|_| BigDecimal::from_str("1.0").unwrap());

            let existing = sqlx::query!(r#"SELECT id FROM bets_extended WHERE id = $1"#, event_id)
                .fetch_optional(self.db.pool())
                .await?;

            if existing.is_some() {
                continue;
            }

            sqlx::query!(
                r#"
                INSERT INTO bets_extended (
                    id, "userId", "marketId", "blockchainBetId", position, amount, shares,
                    odds, status, "createdAt", "updatedAt"
                )
                VALUES ($1, $2, $3, NULL, $4, $5, $6, $7, 'active', NOW(), NOW())
                "#,
                event_id,
                user_uuid,
                market_uuid,
                position,
                amount,
                shares.clone(),
                odds_decimal
            )
            .execute(self.db.pool())
            .await
            .map_err(|e| AppError::Internal(format!("Failed to insert bet: {}", e)))?;

            synced_count += 1;
            info!(
                "Synced bet {} for market {} (market will be auto-updated by trigger)",
                event_id, market_uuid
            );
        }

        info!("Successfully synced {} bets from indexer", synced_count);

        Ok(synced_count)
    }

    pub async fn recalculate_odds_for_existing_bets(&self) -> Result<usize> {
        info!("Recalculating odds for existing bets...");

        let bets_to_update = sqlx::query!(
            r#"
            SELECT 
                b.id,
                b."marketId",
                b.position,
                m."yesPoolSize",
                m."noPoolSize",
                m."totalPoolSize"
            FROM bets_extended b
            JOIN markets_extended m ON b."marketId" = m.id
            WHERE b.odds = '1.0' OR b.odds = '1.00'
            "#
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut updated_count = 0;

        for bet in bets_to_update {
            let yes_pool = bet.yesPoolSize.to_string().parse::<f64>().unwrap_or(1.0);
            let no_pool = bet.noPoolSize.to_string().parse::<f64>().unwrap_or(1.0);
            let total_pool = bet.totalPoolSize.to_string().parse::<f64>().unwrap_or(2.0);

            let calculated_odds = if bet.position.unwrap_or(true) {
                if yes_pool > 0.0 {
                    total_pool / yes_pool
                } else {
                    1.0
                }
            } else if no_pool > 0.0 {
                total_pool / no_pool
            } else {
                1.0
            };

            use bigdecimal::BigDecimal;
            use std::str::FromStr;
            let odds_decimal = BigDecimal::from_str(&format!("{:.2}", calculated_odds))
                .unwrap_or_else(|_| BigDecimal::from_str("1.0").unwrap());

            sqlx::query!(
                r#"UPDATE bets_extended SET odds = $1 WHERE id = $2"#,
                odds_decimal,
                bet.id
            )
            .execute(self.db.pool())
            .await?;

            updated_count += 1;
        }

        info!("Recalculated odds for {} bets", updated_count);

        Ok(updated_count)
    }
}
