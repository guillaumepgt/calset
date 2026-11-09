mod common;

use common::{dummy_payload_timed, dummy_payload_all_day, dummy_payload_all_day_range};
use calset::caldav::CalDavClient;

use wiremock::matchers::{body_string_contains, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};
use uuid::Uuid;

#[tokio::test]
async fn test_ensure_calendar_exists_creates_calendar() {
    let mock_server = MockServer::start().await;

    Mock::given(method("MKCALENDAR"))
        .and(path("/test_user/calendar/"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let client = CalDavClient::new(
        &mock_server.uri(),
        "test_user",
        "test_pass",
        "calendar",
    );
    let result = client.ensure_calendar_exists().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_ensure_calendar_exists_already_exists_conflict() {
    let mock_server = MockServer::start().await;

    Mock::given(method("MKCALENDAR"))
        .and(path("/test_user/calendar/"))
        .respond_with(ResponseTemplate::new(409))
        .mount(&mock_server)
        .await;

    let client = CalDavClient::new(
        &mock_server.uri(),
        "test_user",
        "test_pass",
        "calendar",
    );
    let result = client.ensure_calendar_exists().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_event_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path_regex(r"^/test_user/calendar/[a-f0-9-]+\.ics$"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let caldav_client = CalDavClient::new(
        &mock_server.uri(),
        "test_user",
        "test_pass",
        "calendar",
    );

    let payload = dummy_payload_timed();
    let result = caldav_client.create_event(payload).await;

    let uid = result.unwrap();
    assert!(Uuid::parse_str(&uid).is_ok());
}

#[tokio::test]
async fn test_create_event_failure() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path_regex(r"^/test_user/calendar/[a-f0-9-]+\.ics$"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let caldav_client = CalDavClient::new(
        &mock_server.uri(),
        "test_user",
        "test_pass",
        "calendar",
    );

    let payload = dummy_payload_timed();
    let result = caldav_client.create_event(payload).await;
    
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_all_day_event_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path_regex(r"^/test_user/calendar/[a-f0-9-]+\.ics$"))
        .and(body_string_contains("DTSTART;VALUE=DATE:20261109"))
        .and(body_string_contains("DTEND;VALUE=DATE:20261110"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let caldav_client = CalDavClient::new(
        &mock_server.uri(),
        "test_user",
        "test_pass",
        "calendar",
    );

    let payload = dummy_payload_all_day();
    let result = caldav_client.create_event(payload).await;

    let uid = result.unwrap();
    assert!(Uuid::parse_str(&uid).is_ok());
}

#[tokio::test]
async fn test_create_all_day_range_event_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path_regex(r"^/test_user/calendar/[a-f0-9-]+\.ics$"))
        .and(body_string_contains("DTSTART;VALUE=DATE:20261109"))
        .and(body_string_contains("DTEND;VALUE=DATE:20261111"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let caldav_client = CalDavClient::new(
        &mock_server.uri(),
        "test_user",
        "test_pass",
        "calendar",
    );

    let payload = dummy_payload_all_day_range();
    let result = caldav_client.create_event(payload).await;

    let uid = result.unwrap();
    assert!(Uuid::parse_str(&uid).is_ok());
}