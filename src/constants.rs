use crate::error::AppError;

pub const USDC_DECIMALS: u32 = 6;

pub const USDC_UNIT: u64 = 1_000_000;

pub const MIN_BET_RAW: u64 = USDC_UNIT;

pub const MAX_BET_RAW: u64 = 10_000 * USDC_UNIT;

pub fn usdc_to_raw(amount: f64) -> u64 {
    (amount * USDC_UNIT as f64) as u64
}

pub fn raw_to_usdc(amount: u64) -> f64 {
    amount as f64 / USDC_UNIT as f64
}

pub fn parse_usdc_amount(amount_str: &str) -> Result<u64, AppError> {
    let amount: f64 = amount_str
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid amount format".to_string()))?;

    if amount < 0.0 {
        return Err(AppError::BadRequest(
            "Amount cannot be negative".to_string(),
        ));
    }

    let amount_raw = usdc_to_raw(amount);

    if amount_raw == 0 {
        return Err(AppError::BadRequest(
            "Amount too small (minimum is 0.000001 USDC)".to_string(),
        ));
    }

    Ok(amount_raw)
}

pub fn validate_bet_amount(amount: u64) -> Result<(), AppError> {
    if amount < MIN_BET_RAW {
        return Err(AppError::BadRequest(format!(
            "Minimum bet is {} USDC",
            raw_to_usdc(MIN_BET_RAW)
        )));
    }

    if amount > MAX_BET_RAW {
        return Err(AppError::BadRequest(format!(
            "Maximum bet is {} USDC",
            raw_to_usdc(MAX_BET_RAW)
        )));
    }

    Ok(())
}

pub fn format_usdc_amount(amount_raw: u64) -> String {
    let amount_usdc = raw_to_usdc(amount_raw);
    format!("{:.2} USDC", amount_usdc)
}

pub fn bigdecimal_to_raw(amount: &bigdecimal::BigDecimal) -> u64 {
    amount.to_string().parse::<f64>().unwrap_or(0.0).round() as u64
}

pub fn raw_to_bigdecimal(amount: u64) -> bigdecimal::BigDecimal {
    use std::str::FromStr;
    bigdecimal::BigDecimal::from_str(&amount.to_string())
        .unwrap_or_else(|_| bigdecimal::BigDecimal::from(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usdc_conversions() {
        assert_eq!(usdc_to_raw(1.0), 1_000_000);
        assert_eq!(usdc_to_raw(0.5), 500_000);
        assert_eq!(usdc_to_raw(100.0), 100_000_000);
        assert_eq!(usdc_to_raw(0.000001), 1);

        assert_eq!(raw_to_usdc(1_000_000), 1.0);
        assert_eq!(raw_to_usdc(500_000), 0.5);
        assert_eq!(raw_to_usdc(100_000_000), 100.0);
    }

    #[test]
    fn test_parse_usdc_amount() {
        assert_eq!(parse_usdc_amount("1.5").unwrap(), 1_500_000);
        assert_eq!(parse_usdc_amount("1").unwrap(), 1_000_000);
        assert_eq!(parse_usdc_amount("0.5").unwrap(), 500_000);

        assert!(parse_usdc_amount("-1").is_err());
        assert!(parse_usdc_amount("abc").is_err());
    }

    #[test]
    fn test_validate_bet_amount() {
        assert!(validate_bet_amount(1_000_000).is_ok());
        assert!(validate_bet_amount(10_000_000_000).is_ok());

        assert!(validate_bet_amount(500_000).is_err());
        assert!(validate_bet_amount(11_000_000_000).is_err());
    }

    #[test]
    fn test_format_usdc_amount() {
        assert_eq!(format_usdc_amount(1_000_000), "1.00 USDC");
        assert_eq!(format_usdc_amount(1_500_000), "1.50 USDC");
        assert_eq!(format_usdc_amount(123_456_789), "123.46 USDC");
    }
}
