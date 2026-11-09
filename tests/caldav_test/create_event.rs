use crate::common::*;

use wiremock::matchers::{method, path_regex, body_string_contains};
use wiremock::{Mock, ResponseTemplate};
use uuid::Uuid;

#[tokio::test]
async fn test_create_event_success() {
    let (mock_server, client) = setup_mock_client().await;

    Mock::given(method("PUT"))
        .and(path_regex(r"^/test_user/calendar/[a-f0-9-]+\.ics$"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let payload = dummy_payload_timed();
    let result = client.create_event(payload).await;

    let uid = result.unwrap();
    assert!(Uuid::parse_str(&uid).is_ok());
}

#[tokio::test]
async fn test_create_event_failure() {
    let (mock_server, client) = setup_mock_client().await;

    Mock::given(method("PUT"))
        .and(path_regex(r"^/test_user/calendar/[a-f0-9-]+\.ics$"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let payload = dummy_payload_timed();
    let result = client.create_event(payload).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_all_day_event_success() {
    let (mock_server, client) = setup_mock_client().await;

    Mock::given(method("PUT"))
        .and(path_regex(r"^/test_user/calendar/[a-f0-9-]+\.ics$"))
        .and(body_string_contains("DTSTART;VALUE=DATE:20261109"))
        .and(body_string_contains("DTEND;VALUE=DATE:20261110"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let payload = dummy_payload_all_day();
    let result = client.create_event(payload).await;

    let uid = result.unwrap();
    assert!(Uuid::parse_str(&uid).is_ok());
}

#[tokio::test]
async fn test_create_all_day_range_event_success() {
    let (mock_server, client) = setup_mock_client().await;

    Mock::given(method("PUT"))
        .and(path_regex(r"^/test_user/calendar/[a-f0-9-]+\.ics$"))
        .and(body_string_contains("DTSTART;VALUE=DATE:20261109"))
        .and(body_string_contains("DTEND;VALUE=DATE:20261111"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let payload = dummy_payload_all_day_range();
    let result = client.create_event(payload).await;

    let uid = result.unwrap();
    assert!(Uuid::parse_str(&uid).is_ok());
}