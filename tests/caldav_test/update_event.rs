use crate::common::setup_mock_client;

use wiremock::matchers::{method, path_regex, body_string_contains};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn test_update_event_success() {
    let (mock_server, _client) = setup_mock_client().await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/test_user/calendar/test-event-1.ics$"))
        .and(body_string_contains("BEGIN:VCALENDAR"))
        .and(body_string_contains("BEGIN:VEVENT"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;


}