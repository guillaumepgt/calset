use anyhow::{Context, Result};
use chrono_tz::Tz;
use super::models::AppConfig;

pub fn load_from_env() -> Result<AppConfig> {
    let radicale_url = std::env::var("RADICALE_URL")
        .unwrap_or_else(|_| "http://localhost:5232".to_string());

    let cal_user = std::env::var("CAL_USER")
        .context("CAL_USER environment variable is not set")?;

    let cal_pass = std::env::var("CAL_PASS")
        .context("CAL_PASS environment variable is not set")?;

    let cal_name = std::env::var("CAL_NAME")
        .unwrap_or_else(|_| "agenda".to_string());

    let tz_str = std::env::var("TZ")
        .unwrap_or_else(|_| "Europe/Paris".to_string());

    let timezone: Tz = tz_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", tz_str))?;

    Ok(AppConfig {
        radicale_url,
        cal_user,
        cal_pass,
        cal_name,
        timezone,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_timezone_parse() {
        let tz: Result<Tz, _> = "Europe/Paris".parse();
        assert!(tz.is_ok());
    }

    #[test]
    fn test_missing_env_vars() {
        let old_user = std::env::var("CAL_USER").ok();
        std::env::remove_var("CAL_USER");

        let config_result = load_from_env();
        assert!(config_result.is_err());
        let error_message = config_result.unwrap_err().to_string();
        assert!(error_message.contains("CAL_USER"));

        if let Some(val) = old_user {
            std::env::set_var("CAL_USER", val);
        }
    }
}