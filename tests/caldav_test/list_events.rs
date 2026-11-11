use crate::common::setup_mock_client;

use chrono::TimeZone;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn test_list_events_success() {
    let (mock_server, client) = setup_mock_client().await;

    let xml_response = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
    <D:response>
        <D:href>/caldav/user/cal/1.ics</D:href>
        <D:propstat>
            <D:status>HTTP/1.1 200 OK</D:status>
            <D:prop>
                <D:getetag>"etag1"</D:getetag>
                <C:calendar-data>BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:test-uid
SUMMARY:Test Event
DTSTART:20260115T100000Z
DTEND:20260115T110000Z
END:VEVENT
END:VCALENDAR</C:calendar-data>
            </D:prop>
        </D:propstat>
    </D:response>
</D:multistatus>"#;

    Mock::given(method("REPORT"))
        .and(path("/test_user/calendar/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(xml_response))
        .mount(&mock_server)
        .await;

    let start_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let end_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 31, 23, 59, 59).unwrap();

    let result = client.list_events(start_utc, end_utc).await;
    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].uid, "test-uid");
    assert_eq!(events[0].summary, "Test Event");
}

#[tokio::test]
async fn test_list_events_empty() {
    let (mock_server, client) = setup_mock_client().await;

    let xml_response = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:">
</D:multistatus>"#;

    Mock::given(method("REPORT"))
        .and(path("/test_user/calendar/"))
        .respond_with(ResponseTemplate::new(207).set_body_string(xml_response))
        .mount(&mock_server)
        .await;

    let start_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let end_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 31, 23, 59, 59).unwrap();

    let result = client.list_events(start_utc, end_utc).await;
    assert!(result.is_ok());
    let events = result.unwrap();
    assert!(events.is_empty());
}

#[tokio::test]
async fn test_list_events_server_error() {
    let (mock_server, client) = setup_mock_client().await;

    Mock::given(method("REPORT"))
        .and(path("/test_user/calendar/"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let start_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let end_utc = chrono::Utc.with_ymd_and_hms(2026, 1, 31, 23, 59, 59).unwrap();

    let result = client.list_events(start_utc, end_utc).await;
    assert!(result.is_err());
}
