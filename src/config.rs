use anyhow::{Context, Result};
use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub radicale_url: String,
    pub cal_user: String,
    pub cal_pass: String,
    pub cal_name: String,
    pub timezone: Tz,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
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

        Ok(Self {
            radicale_url,
            cal_user,
            cal_pass,
            cal_name,
            timezone,
        })
    }
}