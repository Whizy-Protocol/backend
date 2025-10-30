use anyhow::Result;
use ethers::prelude::*;
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

abigen!(
    IYieldProtocol,
    r#"[
        function getCurrentApy() external view returns (uint256)
    ]"#,
);

async fn fetch_protocol_apy(provider: &Provider<Http>, adapter_address: &str) -> Result<String> {
    let address: Address = adapter_address.parse()?;
    let contract = IYieldProtocol::new(address, Arc::new(provider.clone()));

    match contract.get_current_apy().call().await {
        Ok(apy_basis_points) => {
            let apy_percent = apy_basis_points.as_u64() as f64 / 100.0;
            Ok(format!("{:.2}", apy_percent))
        }
        Err(e) => Err(anyhow::anyhow!("Failed to call getCurrentApy: {}", e)),
    }
}

pub async fn seed_protocols(pool: &PgPool) -> Result<()> {
    info!("Seeding protocols for HEDERA Testnet...");

    let adapters = vec![
        (
            "aave",
            "Aave",
            1,
            "0xE5774115F8921F37f5941E18cEF87d76764d70aa",
            "12.4",
            3,
            "Lending protocol with isolated markets",
            Some("https://res.cloudinary.com/dutlw7bko/image/upload/v1759079205/protocols/aave_b7jg7k.png"),
        ),
        (
            "morpho",
            "Morpho",
            2,
            "0x37285E4daFcF31e93D9967BA371cA1a66693eF38",
            "6.2",
            4,
            "Optimized lending protocol on top of Aave and Compound",
            Some("https://res.cloudinary.com/dutlw7bko/image/upload/v1759079206/protocols/morpho_ajqsyp.png"),
        ),
        (
            "compound",
            "Compound",
            3,
            "0x3182d2160E054DC25b291d04530F514020684315",
            "4.8",
            2,
            "Algorithmic money market protocol",
            Some("https://res.cloudinary.com/dutlw7bko/image/upload/v1759079205/protocols/compound_q4rsym.png"),
        ),
    ];

    let rpc_url =
        std::env::var("HEDERA_RPC_URL").unwrap_or_else(|_| "https://hashscan.io/testnet".to_string());

    info!("Connecting to HEDERA Testnet RPC: {}", rpc_url);

    let provider_result = Provider::<Http>::try_from(&rpc_url);

    for (
        name,
        display_name,
        protocol_type,
        adapter_address,
        base_apy,
        risk_level,
        description,
        icon_url,
    ) in adapters
    {
        let id = Uuid::new_v4().to_string();

        let apy = match &provider_result {
            Ok(provider) => match fetch_protocol_apy(provider, adapter_address).await {
                Ok(apy_value) => {
                    info!("📊 Fetched real APY for {}: {}%", name, apy_value);
                    apy_value
                }
                Err(e) => {
                    warn!(
                        "Failed to fetch APY for {}: {}. Using default {}%.",
                        name, e, base_apy
                    );
                    base_apy.to_string()
                }
            },
            Err(e) => {
                warn!(
                    "⚠️  RPC connection failed: {}. Using default APY {}%.",
                    e, base_apy
                );
                base_apy.to_string()
            }
        };

        let apy_decimal = apy
            .parse::<f64>()
            .map(|v| {
                bigdecimal::BigDecimal::from_str(&v.to_string())
                    .unwrap_or(bigdecimal::BigDecimal::from(0))
            })
            .unwrap_or(bigdecimal::BigDecimal::from(0));

        sqlx::query!(
            r#"
            INSERT INTO protocols (id, name, "displayName", "protocolType", address, "baseApy", tvl, "riskLevel", "isActive", description, "iconUrl", "createdAt", "updatedAt")
            VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9, $10, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT (name) DO UPDATE SET
                "displayName" = EXCLUDED."displayName",
                "protocolType" = EXCLUDED."protocolType",
                address = EXCLUDED.address,
                "baseApy" = EXCLUDED."baseApy",
                "riskLevel" = EXCLUDED."riskLevel",
                "isActive" = EXCLUDED."isActive",
                description = EXCLUDED.description,
                "iconUrl" = EXCLUDED."iconUrl",
                "updatedAt" = CURRENT_TIMESTAMP
            "#,
            id,
            name,
            display_name,
            protocol_type,
            adapter_address,
            apy_decimal,
            risk_level,
            true,
            description,
            icon_url
        )
        .execute(pool)
        .await?;

        info!(
            "✅ Seeded protocol: {} - {} (type: {}, address: {}, APY: {}%, risk: {})",
            name, display_name, protocol_type, adapter_address, apy, risk_level
        );
    }

    info!("Protocol seeding complete");
    Ok(())
}

pub async fn seed_markets(pool: &PgPool, count: usize) -> Result<()> {
    info!("🌱 Seeding {} markets from Adjacent API...", count);

    let api_key = match std::env::var("ADJACENT_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            warn!("⚠️  ADJACENT_API_KEY not set. Skipping market seeding.");
            return Ok(());
        }
    };

    let seeder = crate::services::MarketSeeder::new(pool.clone(), api_key)?;
    let result = seeder.seed_markets(count).await?;

    info!(
        "Market seeding complete: {} created, {} updated, {} skipped, {} errors",
        result.created, result.updated, result.skipped, result.errors
    );

    Ok(())
}

pub async fn run_all_seeds(pool: &PgPool) -> Result<()> {
    info!("🌱 Running all seeds...");

    match seed_protocols(pool).await {
        Ok(_) => info!("✅ Protocol seeding successful"),
        Err(e) => error!("⚠️  Protocol seeding failed: {}. Continuing...", e),
    }

    let market_count = std::env::var("SEED_MARKET_COUNT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    match seed_markets(pool, market_count).await {
        Ok(_) => info!("✅ Market seeding from Adjacent API successful"),
        Err(e) => error!("⚠️  Market seeding failed: {}. Continuing...", e),
    }

    info!("✅ All seeds complete");
    Ok(())
}
