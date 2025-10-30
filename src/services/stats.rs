use crate::{db::Database, error::Result, models::*};
use sqlx::Row;

pub struct StatsService {
    db: Database,
}

impl StatsService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn get_platform_stats(&self) -> Result<PlatformStats> {
        let stats = sqlx::query(
            r#"
            SELECT
                (SELECT COUNT(*) FROM markets_extended) as total_markets,
                (SELECT COUNT(*) FROM markets_extended WHERE status = 'active') as active_markets,
                (SELECT COUNT(*) FROM markets_extended WHERE status = 'resolved') as resolved_markets,
                (SELECT COUNT(*) FROM bets_extended) as total_bets,
                (SELECT COALESCE(SUM(amount), 0) FROM bets_extended) as total_volume,
                (SELECT COUNT(*) FROM users) as unique_users,
                (SELECT COALESCE(SUM("totalYieldEarned"), 0) FROM markets_extended) as total_yield_earned
            "#,
        )
        .fetch_one(self.db.pool())
        .await?;

        Ok(PlatformStats {
            total_markets: stats.try_get("total_markets")?,
            active_markets: stats.try_get("active_markets")?,
            resolved_markets: stats.try_get("resolved_markets")?,
            total_bets: stats.try_get("total_bets")?,
            total_volume: stats
                .try_get::<bigdecimal::BigDecimal, _>("total_volume")?
                .to_string(),
            unique_users: stats.try_get("unique_users")?,
            total_yield_earned: stats
                .try_get::<bigdecimal::BigDecimal, _>("total_yield_earned")?
                .to_string(),
        })
    }

    pub async fn get_market_stats(&self, market_id: &str) -> Result<MarketStats> {
        let stats = sqlx::query(
            r#"
            SELECT
                $1 as market_id,
                COUNT(*) as total_bets,
                COALESCE(SUM(amount), 0) as total_volume,
                COALESCE(SUM(CASE WHEN position = true THEN amount ELSE 0 END), 0) as yes_volume,
                COALESCE(SUM(CASE WHEN position = false THEN amount ELSE 0 END), 0) as no_volume,
                COUNT(DISTINCT "userId") as unique_bettors
            FROM bets_extended
            WHERE "marketId" = $1
            "#,
        )
        .bind(market_id)
        .fetch_one(self.db.pool())
        .await?;

        let total_volume: bigdecimal::BigDecimal = stats.try_get("total_volume")?;
        let yes_volume: bigdecimal::BigDecimal = stats.try_get("yes_volume")?;
        let no_volume: bigdecimal::BigDecimal = stats.try_get("no_volume")?;

        let yes_percentage = if total_volume > bigdecimal::BigDecimal::from(0) {
            (&yes_volume / &total_volume * bigdecimal::BigDecimal::from(100))
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0)
        } else {
            0.0
        };

        let no_percentage = if total_volume > bigdecimal::BigDecimal::from(0) {
            (&no_volume / &total_volume * bigdecimal::BigDecimal::from(100))
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0)
        } else {
            0.0
        };

        Ok(MarketStats {
            market_id: stats.try_get("market_id")?,
            total_bets: stats.try_get("total_bets")?,
            total_volume: total_volume.to_string(),
            yes_volume: yes_volume.to_string(),
            no_volume: no_volume.to_string(),
            yes_percentage,
            no_percentage,
            unique_bettors: stats.try_get("unique_bettors")?,
        })
    }

    pub async fn get_leaderboard(&self, limit: i64) -> Result<Vec<UserStats>> {
        let leaderboard = sqlx::query(
            r#"
            SELECT
                u.address as user_addr,
                COUNT(b.id) as total_bets,
                COALESCE(SUM(b.amount), 0) as total_wagered,
                COUNT(DISTINCT b."marketId") as markets_participated,
                COUNT(CASE WHEN b.status = 'won' THEN 1 END) as wins,
                COUNT(CASE WHEN b.status = 'lost' THEN 1 END) as losses,
                COUNT(CASE WHEN b.status = 'active' THEN 1 END) as pending,
                COALESCE(SUM(CASE WHEN b.status = 'won' THEN b.payout ELSE 0 END), 0) as total_winnings,
                0 as total_yield_earned
            FROM users u
            LEFT JOIN bets_extended b ON u.id = b."userId"
            GROUP BY u.address
            HAVING COUNT(b.id) > 0
            ORDER BY total_wagered DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;

        let mut result = Vec::new();
        for row in leaderboard {
            result.push(UserStats {
                user_addr: row.try_get("user_addr")?,
                total_bets: row.try_get("total_bets")?,
                total_wagered: row
                    .try_get::<bigdecimal::BigDecimal, _>("total_wagered")?
                    .to_string(),
                markets_participated: row.try_get("markets_participated")?,
                wins: row.try_get("wins")?,
                losses: row.try_get("losses")?,
                pending: row.try_get("pending")?,
                total_winnings: row
                    .try_get::<bigdecimal::BigDecimal, _>("total_winnings")?
                    .to_string(),
                total_yield_earned: "0".to_string(),
            });
        }

        Ok(result)
    }

    pub async fn get_trending_markets(&self, limit: i64) -> Result<Vec<MarketExtended>> {
        let markets = sqlx::query_as::<_, MarketExtended>(
            r#"
            SELECT DISTINCT
                m.id, m."blockchainMarketId", m."marketId", m."adjTicker", m.platform, m.question,
                m.description, m.rules, m.status, m.probability, m.volume, m."openInterest",
                m."endDate", m."resolutionDate", m.result, m.link, m."imageUrl",
                m."totalPoolSize", m."yesPoolSize", m."noPoolSize", m."countYes", m."countNo",
                m."currentYield", m."totalYieldEarned", m."createdAt", m."updatedAt"
            FROM markets_extended m
            LEFT JOIN bets_extended b ON m.id = b."marketId"
            WHERE m.status = 'active'
            GROUP BY m.id
            ORDER BY COUNT(b.id) DESC, m."updatedAt" DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(self.db.pool())
        .await?;

        Ok(markets)
    }
}
