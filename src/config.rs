use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub database_timezone: String,
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub cors_origin: String,
    pub api_key: String,
    pub jwt_secret: String,
    pub base_rpc_url: String,
    pub base_chain_id: u64,
    pub whizy_prediction_market_addr: String,
    pub protocol_selector_addr: String,
    pub usdc_address: String,
    pub access_control_address: String,
    pub aave_adapter_address: String,
    pub compound_adapter_address: String,
    pub morpho_adapter_address: String,
    pub aave_fork_address: String,
    pub compound_fork_address: String,
    pub morpho_fork_address: String,
    pub run_seeds: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

        let database_timezone = env::var("DATABASE_TIMEZONE").unwrap_or_else(|_| "UTC".to_string());

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = env::var("PORT")
            .unwrap_or_else(|_| "3002".to_string())
            .parse::<u16>()
            .context("PORT must be a valid number")?;

        let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        let cors_origin = env::var("CORS_ORIGIN").unwrap_or_else(|_| "*".to_string());

        let api_key = env::var("API_KEY").unwrap_or_else(|_| "dev-api-key".to_string());

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "dev-jwt-secret".to_string());

        let base_rpc_url =
            env::var("HEDERA_RPC_URL").unwrap_or_else(|_| "https://hashscan.io/testnet".to_string());

        let base_chain_id = env::var("HEDERA_CHAIN_ID")
            .unwrap_or_else(|_| "296".to_string())
            .parse::<u64>()
            .unwrap_or(296);

        let whizy_prediction_market_addr = env::var("WHIZY_PREDICTION_MARKET_ADDR")
            .unwrap_or_else(|_| "0x2695CB6da12c6e3C34afd05982607CFd22d40415".to_string());

        let protocol_selector_addr = env::var("PROTOCOL_SELECTOR_ADDR")
            .unwrap_or_else(|_| "0xeEC0774B4296eb132376E9669Dd5F3EEA4aa8A6A".to_string());

        let usdc_address = env::var("USDC_ADDRESS")
            .unwrap_or_else(|_| "0x70dA56284e963dc848D2Ea247664Cbc486dAbd7f".to_string());

        let access_control_address = env::var("ACCESS_CONTROL_ADDRESS")
            .unwrap_or_else(|_| "0xf6A0551512A0aECb19534D9AC35067892063b4ff".to_string());

        let aave_adapter_address = env::var("AAVE_ADAPTER_ADDRESS")
            .unwrap_or_else(|_| "0x98A593E804C70a3fe039f91fF26f31B26A181960".to_string());

        let compound_adapter_address = env::var("COMPOUND_ADAPTER_ADDRESS")
            .unwrap_or_else(|_| "0xA59aDaF04b92b72650BE78f75bF87DAF14331483".to_string());

        let morpho_adapter_address = env::var("MORPHO_ADAPTER_ADDRESS")
            .unwrap_or_else(|_| "0x160c5E4A4D140621c05B43D1547C419bea322Ad1".to_string());

        let aave_fork_address = env::var("AAVE_FORK_ADDRESS")
            .unwrap_or_else(|_| "0x88a1DC41Aa6be6a1113C08F691Eb01d999E2a473".to_string());

        let compound_fork_address = env::var("COMPOUND_FORK_ADDRESS")
            .unwrap_or_else(|_| "0x30dAe39cbeF42971dbc811d2B0fEECd894319DA0".to_string());

        let morpho_fork_address = env::var("MORPHO_FORK_ADDRESS")
            .unwrap_or_else(|_| "0x2D35B90e7E1e03a4D6ED369AeeB3D2BcF3DFb312".to_string());

        let run_seeds = env::var("RUN_SEEDS")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        Ok(Self {
            database_url,
            database_timezone,
            host,
            port,
            log_level,
            cors_origin,
            api_key,
            jwt_secret,
            base_rpc_url,
            base_chain_id,
            whizy_prediction_market_addr,
            protocol_selector_addr,
            usdc_address,
            access_control_address,
            aave_adapter_address,
            compound_adapter_address,
            morpho_adapter_address,
            aave_fork_address,
            compound_fork_address,
            morpho_fork_address,
            run_seeds,
        })
    }
}
