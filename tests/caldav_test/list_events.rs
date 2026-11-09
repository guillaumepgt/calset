use crate::common::setup_mock_client;

use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
use chrono::TimeZone;

#[tokio::test]
async fn test_list_events_success() {
    let (mock_server, client) = setup_mock_client().await;

    Mock::given(method("REPORT"))
        .and(path("/test_user/calendar/"))
        .respond_with(ResponseTemplate::new(207).set_body_string("<response>Event List</response>"))
        .mount(&mock_server)
        .await;

    let start_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
        .unwrap();
    let end_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 31, 23, 59, 59)
        .unwrap();

    let result = client.list_events(start_utc, end_utc).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "<response>Event List</response>");
}

#[tokio::test]
async fn test_list_events_invalid_range() {
    let (_mock_server, client) = setup_mock_client().await;

    let start_utc = chrono::Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0)
        .unwrap();
    let end_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 31, 23, 59, 59)
        .unwrap();

    let result = client.list_events(start_utc, end_utc).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "The start date must be earlier than the end date");
}

#[tokio::test]
async fn test_list_events_server_error() {
    let (mock_server, client) = setup_mock_client().await;

    Mock::given(method("REPORT"))
        .and(path("/test_user/calendar/"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let start_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
        .unwrap();
    let end_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 31, 23, 59, 59)
        .unwrap();

    let result = client.list_events(start_utc, end_utc).await;
    assert!(result.is_err());
}