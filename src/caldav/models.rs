use chrono::{DateTime, NaiveDate, Utc};
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
    pub description: Option<String>,
    pub location: Option<String>,
    pub schedule: EventSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventSchedule {
    Timed {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    AllDay {
        start: NaiveDate,
        end: NaiveDate,
    },
}

impl EventSchedule {
    pub fn timed(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self::Timed { start, end }
    }

    pub fn all_day(date: NaiveDate) -> Self {
        let end = date.succ_opt().expect("date overflow");
        Self::AllDay { start: date, end }
    }

    pub fn all_day_range(start: NaiveDate, end_inclusive: NaiveDate) -> Self {
        let end = end_inclusive.succ_opt().expect("date overflow");
        Self::AllDay { start, end }
    }
}