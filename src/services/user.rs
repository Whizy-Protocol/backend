use crate::{
    db::Database,
    error::{AppError, Result},
    models::*,
};
use sqlx::Row;

pub struct UserService {
    db: Database,
}

impl UserService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn get_user_by_address(&self, address: &str) -> Result<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, address, username, "avatarUrl", "createdAt", "updatedAt"
            FROM users
            WHERE address = $1
            "#,
        )
        .bind(address)
        .fetch_one(self.db.pool())
        .await
        .map_err(|_| AppError::NotFound(format!("User with address {} not found", address)))?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, address, username, "avatarUrl", "createdAt", "updatedAt"
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(self.db.pool())
        .await
        .map_err(|_| AppError::NotFound(format!("User with id {} not found", id)))?;

        Ok(user)
    }

    pub async fn upsert_user(&self, address: &str) -> Result<User> {
        let user_id = uuid::Uuid::new_v4().to_string();

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, address)
            VALUES ($1, $2)
            ON CONFLICT (address)
            DO UPDATE SET "updatedAt" = CURRENT_TIMESTAMP
            RETURNING id, address, username, "avatarUrl", "createdAt", "updatedAt"
            "#,
        )
        .bind(user_id)
        .bind(address)
        .fetch_one(self.db.pool())
        .await?;

        Ok(user)
    }

    pub async fn update_user_profile(
        &self,
        address: &str,
        username: Option<String>,
        avatar_url: Option<String>,
    ) -> Result<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET username = COALESCE($2, username),
                "avatarUrl" = COALESCE($3, "avatarUrl"),
                "updatedAt" = CURRENT_TIMESTAMP
            WHERE address = $1
            RETURNING id, address, username, "avatarUrl", "createdAt", "updatedAt"
            "#,
        )
        .bind(address)
        .bind(username)
        .bind(avatar_url)
        .fetch_one(self.db.pool())
        .await
        .map_err(|_| AppError::NotFound(format!("User with address {} not found", address)))?;

        Ok(user)
    }

    pub async fn get_user_stats(&self, address: &str) -> Result<UserStats> {
        let stats = sqlx::query(
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
            WHERE u.address = $1
            GROUP BY u.address
            "#,
        )
        .bind(address)
        .fetch_one(self.db.pool())
        .await
        .map_err(|_| AppError::NotFound(format!("User with address {} not found", address)))?;

        Ok(UserStats {
            user_addr: stats.try_get("user_addr")?,
            total_bets: stats.try_get("total_bets")?,
            total_wagered: stats
                .try_get::<bigdecimal::BigDecimal, _>("total_wagered")?
                .to_string(),
            markets_participated: stats.try_get("markets_participated")?,
            wins: stats.try_get("wins")?,
            losses: stats.try_get("losses")?,
            pending: stats.try_get("pending")?,
            total_winnings: stats
                .try_get::<bigdecimal::BigDecimal, _>("total_winnings")?
                .to_string(),
            total_yield_earned: "0".to_string(),
        })
    }

    pub async fn get_all_users(&self, limit: i64, offset: i64) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT id, address, username, "avatarUrl", "createdAt", "updatedAt"
            FROM users
            ORDER BY "createdAt" DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(self.db.pool())
        .await?;

        Ok(users)
    }

    pub async fn get_user_count(&self) -> Result<i64> {
        let count: i64 = sqlx::query("SELECT COUNT(*) as count FROM users")
            .fetch_one(self.db.pool())
            .await?
            .try_get("count")?;

        Ok(count)
    }

    pub async fn get_user_with_bets(&self, user_id: &str) -> Result<UserWithBets> {
        let user = self.get_user_by_id(user_id).await?;

        let bets = sqlx::query_as::<_, BetExtended>(
            r#"
            SELECT id, "blockchainBetId", "userId", "marketId", position, amount, odds, status, payout, "createdAt", "updatedAt"
            FROM bets_extended
            WHERE "userId" = $1
            ORDER BY "createdAt" DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(self.db.pool())
        .await?;

        let total_bets = bets.len() as i64;

        Ok(UserWithBets {
            user,
            bets,
            total_bets,
        })
    }
}
