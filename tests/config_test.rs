use calset::config::AppConfig;
use chrono_tz::Tz;

#[test]
fn test_valid_timezone_parse() {
    let tz: Result<Tz, _> = "Europe/Paris".parse();
    assert!(tz.is_ok());
}

#[test]
fn test_missing_env_vars() {
    let old_user = std::env::var("CAL_USER").ok();
    std::env::remove_var("CAL_USER");

    let config_result = AppConfig::from_env();
    assert!(config_result.is_err());
    let error_message = config_result.unwrap_err().to_string();
    assert!(error_message.contains("CAL_USER"));

    if let Some(val) = old_user {
        std::env::set_var("CAL_USER", val);
    }
}