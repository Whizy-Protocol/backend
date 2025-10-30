use anyhow::Result;
use bigdecimal::BigDecimal;
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::adjacent::{AdjacentMarket, AdjacentService};
use super::image_service::ImageService;

pub struct MarketSeeder {
    pool: PgPool,
    adjacent_service: AdjacentService,
    image_service: ImageService,
}

#[allow(dead_code)]
impl MarketSeeder {
    pub fn new(pool: PgPool, api_key: String) -> Result<Self> {
        let adjacent_service = AdjacentService::new(api_key)?;
        let image_service = ImageService::new()?;

        info!("ℹ️  Market seeder will create markets in database.");
        info!("ℹ️  Markets will be synced to blockchain automatically after seeding.");

        Ok(Self {
            pool,
            adjacent_service,
            image_service,
        })
    }

    pub async fn seed_markets(&self, count: usize) -> Result<SeedResult> {
        info!("🌱 Starting market seeding: {} markets requested", count);

        let mut result = SeedResult {
            total_requested: count,
            fetched_from_api: 0,
            created: 0,
            updated: 0,
            skipped: 0,
            errors: 0,
        };

        let existing_questions = self.get_existing_questions().await?;

        let fetch_count = count;
        let api_response = self
            .adjacent_service
            .get_exact_quality_markets(fetch_count)
            .await?;

        result.fetched_from_api = api_response.data.len();
        info!(
            "✅ Fetched {} markets from Adjacent API",
            result.fetched_from_api
        );

        let mut unique_markets = Vec::new();
        let mut seen_questions = existing_questions;

        for market in api_response.data {
            if self.is_question_duplicate(&market.question, &seen_questions) {
                info!(
                    "⏭️  Skipping duplicate/similar question: {}",
                    market.question.chars().take(60).collect::<String>()
                );
                result.skipped += 1;
                continue;
            }

            seen_questions.push(market.question.clone());
            unique_markets.push(market);

            if unique_markets.len() >= count {
                break;
            }
        }

        info!(
            "✅ Found {} unique markets after deduplication",
            unique_markets.len()
        );

        for market in unique_markets {
            if !self.adjacent_service.validate_market(&market) {
                warn!("⚠️ Invalid market data for {}, skipping", market.adj_ticker);
                result.skipped += 1;
                continue;
            }

            match self.create_or_update_market(&market).await {
                Ok(created) => {
                    if created {
                        result.created += 1;
                        info!("✅ Created market: {}", market.adj_ticker);
                    } else {
                        result.updated += 1;
                        info!("📝 Updated market: {}", market.adj_ticker);
                    }
                }
                Err(e) => {
                    error!("❌ Failed to process market {}: {}", market.adj_ticker, e);
                    result.errors += 1;
                }
            }
        }

        info!(
            "🎉 Market seeding complete: {} created, {} updated, {} skipped, {} errors",
            result.created, result.updated, result.skipped, result.errors
        );

        Ok(result)
    }

    async fn get_existing_questions(&self) -> Result<Vec<String>> {
        let questions = sqlx::query_scalar!("SELECT question FROM markets_extended")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .flatten()
            .collect();

        Ok(questions)
    }

    fn is_question_duplicate(&self, question: &str, existing: &[String]) -> bool {
        let normalized_question = self.normalize_question(question);

        for existing_q in existing {
            let normalized_existing = self.normalize_question(existing_q);

            if normalized_question == normalized_existing {
                return true;
            }

            let similarity =
                self.calculate_question_similarity(&normalized_question, &normalized_existing);
            if similarity > 0.8 {
                return true;
            }
        }

        false
    }

    fn normalize_question(&self, question: &str) -> String {
        question
            .to_lowercase()
            .trim()
            .replace("?", "")
            .replace(".", "")
            .replace(",", "")
            .replace("  ", " ")
    }

    fn calculate_question_similarity(&self, q1: &str, q2: &str) -> f64 {
        let words1: std::collections::HashSet<&str> = q1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = q2.split_whitespace().collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let intersection: usize = words1.intersection(&words2).count();
        let union: usize = words1.union(&words2).count();

        if union == 0 {
            return 0.0;
        }

        intersection as f64 / union as f64
    }

    async fn create_or_update_market(&self, api_market: &AdjacentMarket) -> Result<bool> {
        let end_date = chrono::DateTime::parse_from_rfc3339(&api_market.end_date)?.naive_utc();

        let resolution_date = api_market
            .resolution_date
            .as_ref()
            .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
            .map(|d| d.naive_utc());

        let volume = api_market.volume.unwrap_or(0.0);
        let open_interest = api_market.open_interest.unwrap_or(0.0);

        let probability = api_market.probability.round() as i32;

        let image_url = self
            .image_service
            .generate_market_image_with_fallback(&api_market.question)
            .await;

        let existing = sqlx::query!(
            "SELECT id FROM markets_extended WHERE \"adjTicker\" = $1 OR question = $2",
            api_market.adj_ticker,
            api_market.question
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(existing_market) = existing {
            sqlx::query!(
                r#"
                UPDATE markets_extended SET
                    question = $1,
                    description = $2,
                    rules = $3,
                    status = $4,
                    probability = $5,
                    volume = $6,
                    "openInterest" = $7,
                    "endDate" = $8,
                    "resolutionDate" = $9,
                    result = $10,
                    link = $11,
                    "imageUrl" = $12,
                    "updatedAt" = CURRENT_TIMESTAMP
                WHERE id = $13
                "#,
                api_market.question,
                api_market.description,
                api_market.rules,
                api_market.status,
                probability,
                BigDecimal::try_from(volume).unwrap_or_else(|_| BigDecimal::from(0)),
                BigDecimal::try_from(open_interest).unwrap_or_else(|_| BigDecimal::from(0)),
                end_date,
                resolution_date,
                api_market.result,
                api_market.link,
                image_url,
                existing_market.id
            )
            .execute(&self.pool)
            .await?;

            Ok(false)
        } else {
            let id = Uuid::new_v4().to_string();

            sqlx::query!(
                r#"
                INSERT INTO markets_extended (
                    id, "marketId", "adjTicker", platform, question, description, rules,
                    status, probability, volume, "openInterest", "endDate", "resolutionDate",
                    result, link, "imageUrl", "totalPoolSize", "yesPoolSize", "noPoolSize",
                    "countYes", "countNo", "currentYield", "totalYieldEarned",
                    "createdAt", "updatedAt"
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                "#,
                id,
                api_market.market_id,
                api_market.adj_ticker,
                api_market.platform,
                api_market.question,
                api_market.description,
                api_market.rules,
                api_market.status,
                probability,
                BigDecimal::try_from(volume).unwrap_or_else(|_| BigDecimal::from(0)),
                BigDecimal::try_from(open_interest).unwrap_or_else(|_| BigDecimal::from(0)),
                end_date,
                resolution_date,
                api_market.result,
                api_market.link,
                image_url,
                BigDecimal::from(0),
                BigDecimal::from(0),
                BigDecimal::from(0),
                0,
                0,
                BigDecimal::from(0),
                BigDecimal::from(0),
            )
            .execute(&self.pool)
            .await?;

            info!("✅ Created market in DB: {}", id);

            Ok(true)
        }
    }

    pub async fn sync_market(&self, adj_ticker: &str) -> Result<bool> {
        info!("🔄 Syncing market: {}", adj_ticker);

        let api_response = self.adjacent_service.get_market(adj_ticker).await?;
        let market = api_response.data;

        if !self.adjacent_service.validate_market(&market) {
            return Err(anyhow::anyhow!("Invalid market data"));
        }

        self.create_or_update_market(&market).await
    }
}

#[derive(Debug)]
pub struct SeedResult {
    pub total_requested: usize,
    pub fetched_from_api: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub errors: usize,
}
