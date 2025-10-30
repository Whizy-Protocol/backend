use crate::{
    db::Database,
    error::{AppError, Result},
    models::*,
};
use bigdecimal::BigDecimal;
use ethers::prelude::*;
use sqlx::Row;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, warn};

abigen!(
    IYieldProtocol,
    r#"[
        function getCurrentApy() external view returns (uint256)
    ]"#,
);

pub struct ProtocolService {
    db: Database,
}

impl ProtocolService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn get_protocols(&self) -> Result<Vec<Protocol>> {
        let protocols = sqlx::query_as::<_, Protocol>(
            r#"
            SELECT id, name, "protocolType", address, "baseApy", tvl, "riskLevel", "isActive", "iconUrl", "createdAt"
            FROM protocols
            WHERE "isActive" = true
            ORDER BY tvl DESC
            "#,
        )
        .fetch_all(self.db.pool())
        .await?;

        Ok(protocols)
    }

    pub async fn get_protocol_by_address(&self, address: &str) -> Result<Protocol> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, "protocolType", address, "baseApy", tvl, "riskLevel", "isActive", "iconUrl", "createdAt"
            FROM protocols
            WHERE address = $1
            "#,
            address
        )
        .fetch_one(self.db.pool())
        .await
        .map_err(|_| AppError::NotFound(format!("Protocol with address {} not found", address)))?;

        Ok(Protocol {
            id: row.id,
            name: row.name,
            protocol_type: row.protocolType,
            address: row.address,
            base_apy: row.baseApy,
            tvl: row.tvl,
            risk_level: row.riskLevel,
            is_active: row.isActive,
            icon_url: row.iconUrl,
            created_at: row.createdAt,
        })
    }

    pub async fn get_protocol_by_id(&self, id: &str) -> Result<Protocol> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, "protocolType", address, "baseApy", tvl, "riskLevel", "isActive", "iconUrl", "createdAt"
            FROM protocols
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(self.db.pool())
        .await
        .map_err(|_| AppError::NotFound(format!("Protocol with id {} not found", id)))?;

        Ok(Protocol {
            id: row.id,
            name: row.name,
            protocol_type: row.protocolType,
            address: row.address,
            base_apy: row.baseApy,
            tvl: row.tvl,
            risk_level: row.riskLevel,
            is_active: row.isActive,
            icon_url: row.iconUrl,
            created_at: row.createdAt,
        })
    }

    pub async fn upsert_protocol_from_indexer(
        &self,
        protocol_registered: &ProtocolRegistered,
    ) -> Result<String> {
        let address = protocol_registered.protocol_address.clone();
        let name = protocol_registered.name.clone();
        let protocol_type = protocol_registered.protocol_type;
        let risk_level = protocol_registered.risk_level;

        let result = sqlx::query(
            r#"
            INSERT INTO protocols (
                name, protocol_type, address, apy, tvl, risk_level, is_active
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (address)
            DO UPDATE SET
                name = EXCLUDED.name,
                protocol_type = EXCLUDED.protocol_type,
                risk_level = EXCLUDED.risk_level,
                is_active = EXCLUDED.is_active
            RETURNING id
            "#,
        )
        .bind(name)
        .bind(protocol_type)
        .bind(address)
        .bind(BigDecimal::from(0))
        .bind(BigDecimal::from(0))
        .bind(risk_level)
        .bind(true)
        .fetch_one(self.db.pool())
        .await?;

        let id: String = result.try_get("id")?;
        Ok(id)
    }

    pub async fn update_protocol_metrics(&self, address: &str, apy: &str, tvl: &str) -> Result<()> {
        let apy_decimal = BigDecimal::from_str(apy).unwrap_or_else(|_| BigDecimal::from(0));
        let tvl_decimal = BigDecimal::from_str(tvl).unwrap_or_else(|_| BigDecimal::from(0));

        sqlx::query(
            r#"
            UPDATE protocols
            SET apy = $1, tvl = $2
            WHERE address = $3
            "#,
        )
        .bind(apy_decimal)
        .bind(tvl_decimal)
        .bind(address)
        .execute(self.db.pool())
        .await?;

        Ok(())
    }

    pub async fn get_indexer_protocols(&self) -> Result<Vec<ProtocolRegistered>> {
        Ok(vec![])
    }

    pub async fn protocol_exists(&self, address: &str) -> Result<bool> {
        let result =
            sqlx::query("SELECT EXISTS(SELECT 1 FROM protocols WHERE address = $1) as exists")
                .bind(address)
                .fetch_one(self.db.pool())
                .await?;

        let exists: bool = result.try_get("exists")?;
        Ok(exists)
    }

    pub async fn get_best_protocol_by_risk(&self, risk_level: i32) -> Result<Option<Protocol>> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, "protocolType", address, "baseApy", tvl, "riskLevel", "isActive", "iconUrl", "createdAt"
            FROM protocols
            WHERE "riskLevel" = $1 AND "isActive" = true
            ORDER BY "baseApy" DESC, tvl DESC
            LIMIT 1
            "#,
            risk_level
        )
        .fetch_optional(self.db.pool())
        .await?;

        Ok(row.map(|row| Protocol {
            id: row.id,
            name: row.name,
            protocol_type: row.protocolType,
            address: row.address,
            base_apy: row.baseApy,
            tvl: row.tvl,
            risk_level: row.riskLevel,
            is_active: row.isActive,
            icon_url: row.iconUrl,
            created_at: row.createdAt,
        }))
    }

    async fn fetch_apy_from_blockchain(
        provider: &Provider<Http>,
        adapter_address: &str,
    ) -> Result<String> {
        let address: Address = adapter_address
            .parse()
            .map_err(|e| AppError::Internal(format!("Invalid adapter address: {}", e)))?;

        let contract = IYieldProtocol::new(address, Arc::new(provider.clone()));

        match contract.get_current_apy().call().await {
            Ok(apy_basis_points) => {
                let apy_percent = apy_basis_points.as_u64() as f64 / 100.0;
                Ok(format!("{:.2}", apy_percent))
            }
            Err(e) => Err(AppError::Internal(format!(
                "Failed to call getCurrentApy: {}",
                e
            ))),
        }
    }

    pub async fn update_all_apys_from_blockchain(&self, rpc_url: &str) -> Result<usize> {
        info!("🔄 Updating protocol APYs from blockchain...");

        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| AppError::Internal(format!("Failed to connect to RPC: {}", e)))?;

        let protocols = sqlx::query!(
            r#"
            SELECT id, name, address, "baseApy"
            FROM protocols
            WHERE "isActive" = true
            "#
        )
        .fetch_all(self.db.pool())
        .await?;

        let mut updated_count = 0;

        for protocol in protocols {
            let adapter_address = protocol.address.as_deref().unwrap_or("");

            if adapter_address.is_empty() {
                warn!(
                    "⚠️  Protocol {} has no adapter address, skipping",
                    protocol.name
                );
                continue;
            }

            match Self::fetch_apy_from_blockchain(&provider, adapter_address).await {
                Ok(apy_str) => {
                    let apy_decimal =
                        BigDecimal::from_str(&apy_str).unwrap_or_else(|_| BigDecimal::from(0));

                    match sqlx::query!(
                        r#"
                        UPDATE protocols
                        SET "baseApy" = $1, "updatedAt" = CURRENT_TIMESTAMP
                        WHERE id = $2
                        "#,
                        apy_decimal,
                        protocol.id
                    )
                    .execute(self.db.pool())
                    .await
                    {
                        Ok(_) => {
                            info!("✅ Updated {} APY: {}%", protocol.name, apy_str);
                            updated_count += 1;
                        }
                        Err(e) => {
                            error!("❌ Failed to update APY in DB for {}: {}", protocol.name, e);
                        }
                    }
                }
                Err(e) => {
                    let base_apy = protocol.baseApy;
                    warn!(
                        "⚠️  Failed to fetch APY for {} from blockchain: {}. Keeping current: {:?}%",
                        protocol.name, e, base_apy
                    );
                }
            }
        }

        info!("✅ Updated {} protocol APYs from blockchain", updated_count);
        Ok(updated_count)
    }
}
