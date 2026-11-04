use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub radicale_url: String,
    pub cal_user: String,
    pub cal_pass: String,
    pub cal_name: String,
    pub timezone: Tz,
}