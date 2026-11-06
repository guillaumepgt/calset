use calset::caldav::{CreateEventPayload};

pub fn dummy_payload() -> CreateEventPayload {
    CreateEventPayload {
        summary: "Test Event".to_string(),
        description: Some("This is a test event".to_string()),
        location: Some("Test Location".to_string()),
        start: chrono::Utc::now(),
        end: chrono::Utc::now() + chrono::Duration::hours(1),
    }
}