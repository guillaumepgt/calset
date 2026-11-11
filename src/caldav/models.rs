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

#[derive(Debug, Clone, Default)]
pub struct UpdateEventPayload {
    pub summary: Option<String>,
    pub description: Option<Option<String>>,
    pub location: Option<Option<String>>,
    pub schedule: Option<EventSchedule>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub schedule: EventSchedule,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_schedule_constructors() {
        let start = Utc::now();
        let end = start + chrono::Duration::hours(1);
        let timed = EventSchedule::timed(start, end);
        assert_eq!(timed, EventSchedule::Timed { start, end });

        let date = NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
        let all_day = EventSchedule::all_day(date);
        assert_eq!(
            all_day,
            EventSchedule::AllDay {
                start: date,
                end: NaiveDate::from_ymd_opt(2026, 11, 11).unwrap()
            }
        );

        let end_inclusive = NaiveDate::from_ymd_opt(2026, 11, 15).unwrap();
        let all_day_range = EventSchedule::all_day_range(date, end_inclusive);
        assert_eq!(
            all_day_range,
            EventSchedule::AllDay {
                start: date,
                end: NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
            }
        );
    }
}