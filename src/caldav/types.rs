use chrono::{DateTime, Utc};
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct CalDavClient {
    pub(crate) http: Client,
    pub(crate) base_url: String,
    pub(crate) user: String,
    pub(crate) pass: String,
    pub(crate) calendar_name: String,
}

#[derive(Debug, Clone)]
pub struct CreateEventPayload {
    pub summary: String,
    pub description: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}