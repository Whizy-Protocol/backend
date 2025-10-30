use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::debug;

use crate::error::{AppError, Result};

struct BetRow {
    bet_id: Option<i64>,
    position: Option<bool>,
    amount: Option<BigDecimal>,
    timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartDataPoint {
    pub time: i64,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketChartData {
    pub yes_probability: Vec<ChartDataPoint>,
    pub no_probability: Vec<ChartDataPoint>,
    pub yes_volume: Vec<ChartDataPoint>,
    pub no_volume: Vec<ChartDataPoint>,
    pub total_volume: Vec<ChartDataPoint>,
    pub yes_odds: Vec<ChartDataPoint>,
    pub no_odds: Vec<ChartDataPoint>,
    pub bet_count: Vec<ChartDataPoint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformChartData {
    pub total_volume: Vec<ChartDataPoint>,
    pub active_markets: Vec<ChartDataPoint>,
    pub total_users: Vec<ChartDataPoint>,
    pub total_bets: Vec<ChartDataPoint>,
}

pub struct ChartService {
    pool: PgPool,
    database_timezone: String,
}

impl ChartService {
    pub fn new(pool: PgPool, database_timezone: String) -> Self {
        Self {
            pool,
            database_timezone,
        }
    }

    pub async fn get_market_chart_data(
        &self,
        market_id: &str,
        interval: &str,
        from: Option<i64>,
        to: Option<i64>,
    ) -> Result<MarketChartData> {
        let interval_seconds = Self::validate_interval(interval)?;

        let query_str = format!(
            r#"SELECT EXTRACT(EPOCH FROM ("createdAt" AT TIME ZONE '{}' AT TIME ZONE 'UTC'))::BIGINT as created_at, "blockchainMarketId" FROM markets_extended WHERE id = $1 OR "adjTicker" = $1 LIMIT 1"#,
            self.database_timezone
        );

        let market = sqlx::query_as::<_, (Option<i64>, Option<i64>)>(&query_str)
            .bind(market_id)
            .fetch_optional(&self.pool)
            .await?;

        let market_created_at = market
            .as_ref()
            .and_then(|m| m.0)
            .unwrap_or_else(|| chrono::Utc::now().timestamp() - 86400 * 7);

        let blockchain_market_id = market.and_then(|m| m.1).unwrap_or(0i64);

        let to_timestamp = to.unwrap_or_else(|| chrono::Utc::now().timestamp());

        let from_timestamp = from.unwrap_or(market_created_at);

        debug!("Query params: market_id={}, blockchain_market_id={}, from={}, to={}, market_created_at={}", 
            market_id, blockchain_market_id, from_timestamp, to_timestamp, market_created_at);

        let bets_query_str = format!(
            r#"
            SELECT
                "blockchainBetId" as bet_id,
                position,
                amount,
                EXTRACT(EPOCH FROM ("createdAt" AT TIME ZONE '{}' AT TIME ZONE 'UTC'))::BIGINT as timestamp
            FROM bets_extended
            WHERE "marketId" IN (
                SELECT id FROM markets_extended 
                WHERE id = $1 OR "adjTicker" = $1 OR "blockchainMarketId" = $2
            )
            AND EXTRACT(EPOCH FROM ("createdAt" AT TIME ZONE '{}' AT TIME ZONE 'UTC'))::BIGINT BETWEEN $3 AND $4
            ORDER BY "createdAt" ASC
            "#,
            self.database_timezone, self.database_timezone
        );

        use sqlx::Row;
        let bets: Vec<_> = sqlx::query(&bets_query_str)
            .bind(market_id)
            .bind(blockchain_market_id)
            .bind(from_timestamp)
            .bind(to_timestamp)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| BetRow {
                bet_id: row.try_get("bet_id").ok(),
                position: row.try_get("position").ok(),
                amount: row.try_get("amount").ok(),
                timestamp: row.try_get("timestamp").ok(),
            })
            .collect();

        debug!("Fetched {} bets for chart data", bets.len());

        let actual_start_timestamp = if from.is_some() {
            from_timestamp
        } else if bets.is_empty() {
            market_created_at
        } else {
            bets.first()
                .and_then(|b| b.timestamp)
                .unwrap_or(market_created_at)
        };

        if !bets.is_empty() {
            if let Some(first_bet) = bets.first() {
                debug!(
                    "First bet: id={:?}, amount={:?}, position={:?}, timestamp={:?}",
                    first_bet.bet_id, first_bet.amount, first_bet.position, first_bet.timestamp
                );
                debug!(
                    "Chart will start from first bet timestamp: {}",
                    first_bet.timestamp.unwrap_or(0)
                );
            }
            if let Some(last_bet) = bets.last() {
                debug!("Last bet timestamp: {:?}", last_bet.timestamp);
            }
        }

        let start_bucket = (actual_start_timestamp / interval_seconds) * interval_seconds;
        let end_bucket = (to_timestamp / interval_seconds) * interval_seconds;

        let mut times: Vec<i64> = Vec::new();
        let mut current = start_bucket;
        while current <= end_bucket {
            times.push(current);
            current += interval_seconds;
        }

        let mut time_buckets: HashMap<i64, BucketData> = HashMap::new();
        let mut yes_total: i64 = 0;
        let mut no_total: i64 = 0;

        for bet in bets {
            let bucket_time = (bet.timestamp.unwrap_or(0) / interval_seconds) * interval_seconds;
            let entry = time_buckets
                .entry(bucket_time)
                .or_insert_with(|| BucketData {
                    yes_volume: 0,
                    no_volume: 0,
                    yes_count: 0,
                    no_count: 0,
                    yes_total: 0,
                    no_total: 0,
                });

            let amount_i64: i64 = bet
                .amount
                .map(|a| {
                    let amount_str = a.to_string();

                    let parsed = amount_str
                        .split('.')
                        .next()
                        .and_then(|s| s.parse::<i64>().ok())
                        .unwrap_or(0);
                    debug!("Parsing amount: {} -> {}", amount_str, parsed);
                    parsed
                })
                .unwrap_or(0);

            let is_yes_position = bet.position.unwrap_or(false);

            if is_yes_position {
                entry.yes_volume += amount_i64;
                entry.yes_count += 1;
                yes_total += amount_i64;
            } else {
                entry.no_volume += amount_i64;
                entry.no_count += 1;
                no_total += amount_i64;
            }

            entry.yes_total = yes_total;
            entry.no_total = no_total;
        }

        let mut yes_probability = Vec::new();
        let mut no_probability = Vec::new();
        let mut yes_volume = Vec::new();
        let mut no_volume = Vec::new();
        let mut total_volume = Vec::new();
        let mut yes_odds = Vec::new();
        let mut no_odds = Vec::new();
        let mut bet_count = Vec::new();

        let mut running_yes_pool: i64 = 0;
        let mut running_no_pool: i64 = 0;

        for time in times {
            let bucket = time_buckets.get(&time);

            let bet_count_in_period: i32;

            if let Some(b) = bucket {
                running_yes_pool = b.yes_total;
                running_no_pool = b.no_total;
                bet_count_in_period = b.yes_count + b.no_count;
            } else {
                bet_count_in_period = 0;
            }

            let total_pool = running_yes_pool + running_no_pool;

            let yes_prob = if total_pool > 0 {
                running_yes_pool as f64 / total_pool as f64
            } else {
                0.5
            };
            let no_prob = 1.0 - yes_prob;

            yes_probability.push(ChartDataPoint {
                time,
                value: yes_prob,
            });
            no_probability.push(ChartDataPoint {
                time,
                value: no_prob,
            });

            yes_volume.push(ChartDataPoint {
                time,
                value: running_yes_pool as f64,
            });
            no_volume.push(ChartDataPoint {
                time,
                value: running_no_pool as f64,
            });
            total_volume.push(ChartDataPoint {
                time,
                value: total_pool as f64,
            });

            let yes_odd = if yes_prob > 0.0 { 1.0 / yes_prob } else { 2.0 };
            let no_odd = if no_prob > 0.0 { 1.0 / no_prob } else { 2.0 };

            yes_odds.push(ChartDataPoint {
                time,
                value: yes_odd,
            });
            no_odds.push(ChartDataPoint {
                time,
                value: no_odd,
            });

            bet_count.push(ChartDataPoint {
                time,
                value: bet_count_in_period as f64,
            });
        }

        Ok(MarketChartData {
            yes_probability,
            no_probability,
            yes_volume,
            no_volume,
            total_volume,
            yes_odds,
            no_odds,
            bet_count,
        })
    }

    fn validate_interval(interval: &str) -> Result<i64> {
        let seconds = match interval {
            "1m" => 60,
            "5m" => 300,
            "15m" => 900,
            "1h" => 3600,
            "4h" => 14400,
            "1d" => 86400,
            _ => {
                return Err(AppError::BadRequest(format!(
                    "Invalid interval '{}'. Allowed values: 1m, 5m, 15m, 1h, 4h, 1d",
                    interval
                )))
            }
        };
        Ok(seconds)
    }

    pub async fn get_platform_chart_data(&self, days: i64) -> Result<PlatformChartData> {
        let start_date = Utc::now() - Duration::days(days);

        let volume_data = sqlx::query!(
            r#"
            SELECT 
                DATE_TRUNC('day', "createdAt") as day,
                COALESCE(SUM(amount), 0) as total_volume
            FROM bets_extended
            WHERE "createdAt" >= $1
            GROUP BY day
            ORDER BY day ASC
            "#,
            start_date.naive_utc()
        )
        .fetch_all(&self.pool)
        .await?;

        let total_volume: Vec<ChartDataPoint> = volume_data
            .into_iter()
            .filter_map(|row| {
                let timestamp = row.day?;
                let volume = row.total_volume?.to_string().parse::<f64>().ok()?;
                Some(ChartDataPoint {
                    time: DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc).timestamp(),
                    value: volume,
                })
            })
            .collect();

        let markets_data = sqlx::query!(
            r#"
            SELECT 
                DATE_TRUNC('day', "createdAt") as day,
                COUNT(*) as count
            FROM markets_extended
            WHERE "createdAt" >= $1 AND status = 'active'
            GROUP BY day
            ORDER BY day ASC
            "#,
            start_date.naive_utc()
        )
        .fetch_all(&self.pool)
        .await?;

        let active_markets: Vec<ChartDataPoint> = markets_data
            .into_iter()
            .filter_map(|row| {
                let timestamp = row.day?;
                let count = row.count? as f64;
                Some(ChartDataPoint {
                    time: DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc).timestamp(),
                    value: count,
                })
            })
            .collect();

        let users_data = sqlx::query!(
            r#"
            SELECT 
                DATE_TRUNC('day', "createdAt") as day,
                COUNT(*) as new_users
            FROM users
            WHERE "createdAt" >= $1
            GROUP BY day
            ORDER BY day ASC
            "#,
            start_date.naive_utc()
        )
        .fetch_all(&self.pool)
        .await?;

        let mut cumulative_users = 0;
        let total_users: Vec<ChartDataPoint> = users_data
            .into_iter()
            .filter_map(|row| {
                let timestamp = row.day?;
                cumulative_users += row.new_users? as i32;
                Some(ChartDataPoint {
                    time: DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc).timestamp(),
                    value: cumulative_users as f64,
                })
            })
            .collect();

        let bets_data = sqlx::query!(
            r#"
            SELECT 
                DATE_TRUNC('day', "createdAt") as day,
                COUNT(*) as bet_count
            FROM bets_extended
            WHERE "createdAt" >= $1
            GROUP BY day
            ORDER BY day ASC
            "#,
            start_date.naive_utc()
        )
        .fetch_all(&self.pool)
        .await?;

        let mut cumulative_bets = 0;
        let total_bets: Vec<ChartDataPoint> = bets_data
            .into_iter()
            .filter_map(|row| {
                let timestamp = row.day?;
                cumulative_bets += row.bet_count? as i32;
                Some(ChartDataPoint {
                    time: DateTime::<Utc>::from_naive_utc_and_offset(timestamp, Utc).timestamp(),
                    value: cumulative_bets as f64,
                })
            })
            .collect();

        Ok(PlatformChartData {
            total_volume,
            active_markets,
            total_users,
            total_bets,
        })
    }
}

struct BucketData {
    yes_volume: i64,
    no_volume: i64,
    yes_count: i32,
    no_count: i32,
    yes_total: i64,
    no_total: i64,
}
