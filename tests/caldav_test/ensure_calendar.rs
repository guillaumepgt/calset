use crate::common::setup_mock_client;

use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn test_ensure_calendar_exists_creates_calendar() {
    let (mock_server, client) = setup_mock_client().await;

    Mock::given(method("MKCALENDAR"))
        .and(path("/test_user/calendar/"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&mock_server)
        .await;

    let result = client.ensure_calendar_exists().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_ensure_calendar_exists_already_exists_conflict() {
    let (mock_server, client) = setup_mock_client().await;

    Mock::given(method("MKCALENDAR"))
        .and(path("/test_user/calendar/"))
        .respond_with(ResponseTemplate::new(409))
        .mount(&mock_server)
        .await;

    let result = client.ensure_calendar_exists().await;
    assert!(result.is_ok());
}