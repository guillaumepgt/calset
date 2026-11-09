use calset::caldav::{CalDavClient, CreateEventPayload, EventSchedule};
use chrono::{NaiveDate};
use wiremock::MockServer;

pub async fn setup_mock_client() -> (MockServer, CalDavClient) {
    let mock_server = MockServer::start().await;

    let client = CalDavClient::new(
        &mock_server.uri(),
        "test_user",
        "test_pass",
        "calendar",
    );

    (mock_server, client)
    
}

pub fn dummy_payload_timed() -> CreateEventPayload {
    CreateEventPayload {
        summary: "Test Event".to_string(),
        description: Some("This is a test event".to_string()),
        location: Some("Test Location".to_string()),
        schedule: EventSchedule::timed(
            chrono::Utc::now(),
            chrono::Utc::now() + chrono::Duration::hours(1),
        ),
    }
}

pub fn dummy_payload_all_day() -> CreateEventPayload {
    CreateEventPayload {
        summary: "Test Event".to_string(),
        description: Some("This is a test event".to_string()),
        location: Some("Test Location".to_string()),
        schedule: EventSchedule::all_day(
            NaiveDate::from_ymd_opt(2026,11,9).unwrap(),
        ),
    }
}

pub fn dummy_payload_all_day_range() -> CreateEventPayload {
    CreateEventPayload {
        summary: "Test Event".to_string(),
        description: Some("This is a test event".to_string()),
        location: Some("Test Location".to_string()),
        schedule: EventSchedule::all_day_range(
            NaiveDate::from_ymd_opt(2026,11,9).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11,10).unwrap(),
        ),
    }
}