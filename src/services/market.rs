use crate::{
    db::Database,
    error::{AppError, Result},
    models::*,
};
use bigdecimal::BigDecimal;
use sqlx::Row;

pub struct MarketService {
    db: Database,
}

impl MarketService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn get_markets(&self, params: MarketQueryParams) -> Result<MarketResponse> {
        let status_filter = match params.status {
            MarketStatus::Active => "WHERE m.status = 'active'",
            MarketStatus::Resolved => "WHERE m.status = 'resolved'",
            MarketStatus::All => "",
        };

        let sort_column = match params.sort_by {
            MarketSortBy::EndTime => "m.\"endDate\"",
            MarketSortBy::TransactionVersion => "m.\"createdAt\"",
        };

        let order = match params.order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };

        let count_query = format!(
            "SELECT COUNT(*) as count FROM markets_extended m {}",
            status_filter
        );
        let total: i64 = sqlx::query(&count_query)
            .fetch_one(self.db.pool())
            .await?
            .try_get("count")?;

        let query = format!(
            r#"
            SELECT 
                id, "blockchainMarketId", "marketId", "adjTicker", platform, question, 
                description, rules, status, probability, volume, "openInterest",
                "endDate", "resolutionDate", result, link, "imageUrl",
                "totalPoolSize", "yesPoolSize", "noPoolSize", "countYes", "countNo",
                "currentYield", "totalYieldEarned", "createdAt", "updatedAt"
            FROM markets_extended m
            {}
            ORDER BY {} {}
            LIMIT $1 OFFSET $2
            "#,
            status_filter, sort_column, order
        );

        let mut markets = sqlx::query_as::<_, MarketExtended>(&query)
            .bind(params.limit)
            .bind(params.offset)
            .fetch_all(self.db.pool())
            .await?;

        let best_apy = self.get_best_protocol_apy().await.unwrap_or(5.0);
        for market in &mut markets {
            market.calculate_total_yield_until_end(best_apy);
        }

        Ok(MarketResponse {
            data: markets,
            meta: PaginationMeta {
                total,
                limit: params.limit,
                offset: params.offset,
                has_more: params.offset + params.limit < total,
            },
        })
    }

    pub async fn get_market_by_id(&self, id: &str) -> Result<MarketExtended> {
        let mut market = sqlx::query_as::<_, MarketExtended>(
            r#"
            SELECT 
                id, "blockchainMarketId", "marketId", "adjTicker", platform, question, 
                description, rules, status, probability, volume, "openInterest",
                "endDate", "resolutionDate", result, link, "imageUrl",
                "totalPoolSize", "yesPoolSize", "noPoolSize", "countYes", "countNo",
                "currentYield", "totalYieldEarned", "createdAt", "updatedAt"
            FROM markets_extended
            WHERE id = $1 OR "marketId" = $1
            "#,
        )
        .bind(id)
        .fetch_one(self.db.pool())
        .await
        .map_err(|_| AppError::NotFound(format!("Market with id {} not found", id)))?;

        let best_apy = self.get_best_protocol_apy().await.unwrap_or(5.0);
        market.calculate_total_yield_until_end(best_apy);

        Ok(market)
    }

    pub async fn get_market_by_blockchain_id(
        &self,
        blockchain_id: &str,
    ) -> Result<Option<MarketExtended>> {
        let blockchain_id_i64 = blockchain_id
            .parse::<i64>()
            .map_err(|_| AppError::BadRequest("Invalid blockchain market ID".to_string()))?;

        let mut market = sqlx::query_as::<_, MarketExtended>(
            r#"
            SELECT 
                id, "blockchainMarketId", "marketId", "adjTicker", platform, question, 
                description, rules, status, probability, volume, "openInterest",
                "endDate", "resolutionDate", result, link, "imageUrl",
                "totalPoolSize", "yesPoolSize", "noPoolSize", "countYes", "countNo",
                "currentYield", "totalYieldEarned", "createdAt", "updatedAt"
            FROM markets_extended
            WHERE "blockchainMarketId" = $1
            "#,
        )
        .bind(blockchain_id_i64)
        .fetch_optional(self.db.pool())
        .await?;

        if let Some(ref mut m) = market {
            let best_apy = self.get_best_protocol_apy().await.unwrap_or(5.0);
            m.calculate_total_yield_until_end(best_apy);
        }

        Ok(market)
    }

    pub async fn upsert_market_from_indexer(
        &self,
        market_created: &MarketCreated,
    ) -> Result<String> {
        let blockchain_market_id = market_created.market_id.clone().parse::<i64>().unwrap_or(0);
        let question = market_created.question.clone();
        let end_time = market_created.end_time.clone();

        let timestamp_secs = end_time
            .parse::<i64>()
            .unwrap_or_else(|_| chrono::Utc::now().timestamp());
        let end_date = chrono::DateTime::from_timestamp(timestamp_secs, 0)
            .map(|dt| dt.naive_utc())
            .unwrap_or_else(|| chrono::Utc::now().naive_utc());

        let existing = sqlx::query!(
            r#"SELECT id FROM markets_extended WHERE question = $1 AND "blockchainMarketId" IS NULL LIMIT 1"#,
            question
        )
        .fetch_optional(self.db.pool())
        .await?;

        if let Some(existing_market) = existing {
            sqlx::query!(
                r#"
                UPDATE markets_extended
                SET "blockchainMarketId" = $1, "marketId" = $2, "updatedAt" = CURRENT_TIMESTAMP
                WHERE id = $3
                "#,
                blockchain_market_id,
                market_created.market_id.clone(),
                existing_market.id
            )
            .execute(self.db.pool())
            .await?;

            return Ok(existing_market.id);
        }

        let result = sqlx::query(
            r#"
            INSERT INTO markets_extended (
                "blockchainMarketId", "marketId", question, description, status, platform,
                probability, volume, "totalPoolSize", "yesPoolSize", "noPoolSize",
                "countYes", "countNo", "currentYield", "totalYieldEarned", "endDate", "openInterest"
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ON CONFLICT ("blockchainMarketId") 
            DO UPDATE SET
                question = EXCLUDED.question,
                "updatedAt" = CURRENT_TIMESTAMP
            RETURNING id
            "#,
        )
        .bind(blockchain_market_id)
        .bind(market_created.market_id.clone())
        .bind(question.clone())
        .bind(Some(format!("Prediction market for: {}", question)))
        .bind("active")
        .bind("base")
        .bind(50)
        .bind(BigDecimal::from(0))
        .bind(BigDecimal::from(0))
        .bind(BigDecimal::from(0))
        .bind(BigDecimal::from(0))
        .bind(0)
        .bind(0)
        .bind(BigDecimal::from(0))
        .bind(BigDecimal::from(0))
        .bind(end_date)
        .bind(BigDecimal::from(0))
        .fetch_one(self.db.pool())
        .await?;

        let id: String = result.try_get("id")?;
        Ok(id)
    }

    pub async fn resolve_market(&self, blockchain_market_id: &str, outcome: bool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE markets_extended
            SET status = 'resolved', result = $2, "updatedAt" = CURRENT_TIMESTAMP
            WHERE "marketId" = $1
            "#,
        )
        .bind(blockchain_market_id)
        .bind(outcome)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    pub async fn recalculate_market_stats(&self, market_id: &str) -> Result<()> {
        let stats = sqlx::query(
            r#"
            SELECT 
                COALESCE(SUM(CASE WHEN position = true THEN amount ELSE 0 END), 0) as yes_volume,
                COALESCE(SUM(CASE WHEN position = false THEN amount ELSE 0 END), 0) as no_volume,
                COALESCE(COUNT(CASE WHEN position = true THEN 1 END), 0) as yes_count,
                COALESCE(COUNT(CASE WHEN position = false THEN 1 END), 0) as no_count,
                COALESCE(SUM(amount), 0) as total_volume
            FROM bets_extended
            WHERE "marketId" = $1 AND status = 'active'
            "#,
        )
        .bind(market_id)
        .fetch_one(self.db.pool())
        .await?;

        let yes_volume: BigDecimal = stats.try_get("yes_volume")?;
        let no_volume: BigDecimal = stats.try_get("no_volume")?;
        let yes_count: i64 = stats.try_get("yes_count")?;
        let no_count: i64 = stats.try_get("no_count")?;

        let volume = &yes_volume + &no_volume;

        let probability = if volume > BigDecimal::from(0) {
            ((&yes_volume / &volume) * BigDecimal::from(100))
                .to_string()
                .parse::<i32>()
                .unwrap_or(50)
        } else {
            50
        };

        sqlx::query(
            r#"
            UPDATE markets_extended
            SET 
                "yesPoolSize" = $2,
                "noPoolSize" = $3,
                "countYes" = $4,
                "countNo" = $5,
                volume = $6,
                "totalPoolSize" = $7,
                probability = $8,
                "updatedAt" = CURRENT_TIMESTAMP
            WHERE id = $1
            "#,
        )
        .bind(market_id)
        .bind(&yes_volume)
        .bind(&no_volume)
        .bind(yes_count as i32)
        .bind(no_count as i32)
        .bind(&volume)
        .bind(&volume)
        .bind(probability)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    pub async fn get_indexer_markets(&self) -> Result<Vec<MarketCreated>> {
        Ok(vec![])
    }

    async fn get_best_protocol_apy(&self) -> Result<f64> {
        let result = sqlx::query!(
            r#"
            SELECT MAX("baseApy") as max_apy
            FROM protocols
            WHERE "isActive" = true
            "#
        )
        .fetch_one(self.db.pool())
        .await?;

        let max_apy = result
            .max_apy
            .and_then(|apy| apy.to_string().parse::<f64>().ok())
            .unwrap_or(5.0);

        Ok(max_apy)
    }
}
