use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

const DEFAULT_API_BASE_URL: &str = "https://api.data.adj.news/api";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjacentMarket {
    pub adj_ticker: String,
    pub market_id: String,
    pub platform: String,
    pub question: String,
    pub description: Option<String>,
    pub rules: Option<String>,
    pub status: String,
    pub status_details: Option<StatusDetails>,
    pub probability: f64,
    pub volume: Option<f64>,
    pub open_interest: Option<f64>,
    pub end_date: String,
    pub resolution_date: Option<String>,
    pub result: Option<bool>,
    pub link: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusDetails {
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdjacentApiResponse<T> {
    pub data: T,
    pub meta: ApiMeta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiMeta {
    pub count: usize,
    pub limit: usize,
    pub offset: usize,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_fetched: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub efficiency: Option<i32>,
}

pub struct AdjacentService {
    client: Client,
    api_key: String,
    base_url: String,
}

#[allow(dead_code)]
impl AdjacentService {
    pub fn new(api_key: String) -> Result<Self> {
        let base_url = std::env::var("ADJACENT_API_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string());

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }

    pub async fn get_markets(
        &self,
        limit: usize,
        offset: usize,
        sort_by: &str,
        sort_dir: &str,
    ) -> Result<AdjacentApiResponse<Vec<AdjacentMarket>>> {
        let url = format!("{}/markets", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .query(&[
                ("limit", limit.to_string()),
                ("offset", offset.to_string()),
                ("sort_by", sort_by.to_string()),
                ("sort_dir", sort_dir.to_string()),
            ])
            .send()
            .await
            .context("Failed to fetch markets from Adjacent API")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Adjacent API returned error: {}",
                response.status()
            ));
        }

        let api_response: AdjacentApiResponse<Vec<AdjacentMarket>> = response
            .json()
            .await
            .context("Failed to parse Adjacent API response")?;

        Ok(api_response)
    }

    pub async fn get_market(
        &self,
        adj_ticker: &str,
    ) -> Result<AdjacentApiResponse<AdjacentMarket>> {
        let url = format!("{}/markets/{}", self.base_url, adj_ticker);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .context(format!(
                "Failed to fetch market {} from Adjacent API",
                adj_ticker
            ))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Adjacent API returned error: {} for market {}",
                response.status(),
                adj_ticker
            ));
        }

        let api_response: AdjacentApiResponse<AdjacentMarket> = response
            .json()
            .await
            .context("Failed to parse Adjacent API response")?;

        Ok(api_response)
    }

    pub async fn get_quality_markets(
        &self,
        target_count: usize,
        min_desc_length: usize,
    ) -> Result<AdjacentApiResponse<Vec<AdjacentMarket>>> {
        info!(
            "🎯 Targeting {} quality markets (active + description >{}  chars)",
            target_count, min_desc_length
        );

        let mut quality_markets = Vec::new();
        let mut offset = 0;
        let batch_size = 50;
        let mut total_fetched = 0;
        let max_attempts = 10;
        let mut attempts = 0;

        while quality_markets.len() < target_count && attempts < max_attempts {
            attempts += 1;
            info!(
                "   📡 Batch {}: Fetching {} markets (offset: {})",
                attempts, batch_size, offset
            );

            let url = format!("{}/markets", self.base_url);
            let batch_response_raw = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .query(&[
                    ("limit", batch_size.to_string()),
                    ("offset", offset.to_string()),
                    ("sort", "updated_at:desc".to_string()),
                    ("platform", "polymarket".to_string()),
                    ("market_type", "binary".to_string()),
                    ("status", "active".to_string()),
                ])
                .send()
                .await
                .context("Failed to fetch markets from Adjacent API")?;

            if !batch_response_raw.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Adjacent API returned error: {}",
                    batch_response_raw.status()
                ));
            }

            let batch_response: AdjacentApiResponse<Vec<AdjacentMarket>> = batch_response_raw
                .json()
                .await
                .context("Failed to parse Adjacent API response")?;

            if batch_response.data.is_empty() {
                info!("   ⚠️  No more markets available from API");
                break;
            }

            total_fetched += batch_response.data.len();

            let batch_quality: Vec<AdjacentMarket> = batch_response
                .data
                .into_iter()
                .filter(|market| {
                    if market.question.trim().is_empty() {
                        return false;
                    }

                    let question_lower = market.question.to_lowercase();
                    let yes_count = question_lower.matches(",yes ").count();
                    let no_count = question_lower.matches(",no ").count();
                    if yes_count + no_count >= 2 {
                        return false;
                    }

                    if market
                        .description
                        .as_ref()
                        .map(|d| d.trim().len())
                        .unwrap_or(0)
                        <= min_desc_length
                    {
                        return false;
                    }

                    if let Ok(end_date) = chrono::DateTime::parse_from_rfc3339(&market.end_date) {
                        if end_date.timestamp() <= chrono::Utc::now().timestamp() {
                            return false;
                        }
                    } else {
                        return false;
                    }

                    true
                })
                .collect();

            let batch_count = batch_quality.len();
            quality_markets.extend(batch_quality);

            info!(
                "   ✅ Found {} quality markets in this batch ({}/{} total)",
                batch_count,
                quality_markets.len(),
                target_count
            );

            if quality_markets.len() >= target_count {
                quality_markets.truncate(target_count);
                break;
            }

            offset += batch_size;

            if attempts < max_attempts {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }

        let count = quality_markets.len();
        let efficiency = if total_fetched > 0 {
            ((count as f64 / total_fetched as f64) * 100.0) as i32
        } else {
            0
        };

        info!(
            "🎉 Final result: {}/{} quality markets from {} total fetched ({}% efficiency)",
            count, target_count, total_fetched, efficiency
        );

        Ok(AdjacentApiResponse {
            data: quality_markets,
            meta: ApiMeta {
                count,
                limit: target_count,
                offset: 0,
                has_more: false,
                total_fetched: Some(total_fetched),
                efficiency: Some(efficiency),
            },
        })
    }

    pub async fn get_exact_quality_markets(
        &self,
        target_count: usize,
    ) -> Result<AdjacentApiResponse<Vec<AdjacentMarket>>> {
        info!("🎯 Attempting to find {} quality markets...", target_count);

        let mut result = self.get_quality_markets(target_count, 20).await?;

        if result.data.len() < target_count {
            info!(
                "🔄 Only found {}/{} with 20+ chars. Trying 50+ chars...",
                result.data.len(),
                target_count
            );
            result = self.get_quality_markets(target_count, 50).await?;

            if result.data.len() < target_count {
                info!(
                    "🔄 Only found {}/{} with 50+ chars. Trying 20+ chars...",
                    result.data.len(),
                    target_count
                );
                result = self.get_quality_markets(target_count, 20).await?;

                if result.data.len() < target_count {
                    info!(
                        "🔄 Only found {}/{} with 20+ chars. Accepting any active markets...",
                        result.data.len(),
                        target_count
                    );
                    result = self.get_quality_markets(target_count, 0).await?;
                }
            }
        }

        Ok(result)
    }

    pub fn validate_market(&self, market: &AdjacentMarket) -> bool {
        if market.adj_ticker.is_empty() || market.market_id.is_empty() || market.question.is_empty()
        {
            return false;
        }

        if chrono::DateTime::parse_from_rfc3339(&market.end_date).is_err() {
            return false;
        }

        if market.probability < 0.0 || market.probability > 100.0 {
            return false;
        }

        true
    }
}
