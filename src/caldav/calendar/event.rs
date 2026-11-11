use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use icalendar::{Calendar, CalendarComponent, Component, DatePerhapsTime, Event, EventLike};
use std::str::FromStr;

use crate::caldav::models::{CalendarEvent, CreateEventPayload, EventSchedule, UpdateEventPayload};

fn calendar_datetime_to_utc(cdt: icalendar::CalendarDateTime) -> DateTime<Utc> {
    match cdt {
        icalendar::CalendarDateTime::Utc(dt) => dt,
        icalendar::CalendarDateTime::WithTimezone { date_time, tzid } => {
            if let Ok(tz) = tzid.parse::<Tz>() {
                use chrono::TimeZone;
                match tz.from_local_datetime(&date_time) {
                    chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
                    chrono::LocalResult::Ambiguous(dt, _) => dt.with_timezone(&Utc),
                    chrono::LocalResult::None => {
                        DateTime::<Utc>::from_naive_utc_and_offset(date_time, Utc)
                    }
                }
            } else {
                DateTime::<Utc>::from_naive_utc_and_offset(date_time, Utc)
            }
        }
        icalendar::CalendarDateTime::Floating(naive_dt) => {
            DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc)
        }
    }
}

pub(crate) fn parse_vevent(event: &icalendar::Event) -> Result<CalendarEvent> {
    let uid = event
        .get_uid()
        .context("Missing UID in VEVENT")?
        .to_string();

    let summary = event.get_summary().unwrap_or_default().to_string();
    let description = event.get_description().map(str::to_string);
    let location = event.get_location().map(str::to_string);

    let start = event
        .get_start()
        .context("Missing DTSTART in VEVENT")?;

    let schedule = match start {
        DatePerhapsTime::DateTime(start_cdt) => {
            let start_utc = calendar_datetime_to_utc(start_cdt);
            let end_utc = match event.get_end() {
                Some(DatePerhapsTime::DateTime(end_cdt)) => calendar_datetime_to_utc(end_cdt),
                _ => start_utc,
            };
            EventSchedule::Timed {
                start: start_utc,
                end: end_utc,
            }
        }
        DatePerhapsTime::Date(start_date) => {
            let end_date = match event.get_end() {
                Some(DatePerhapsTime::Date(d)) => d,
                _ => start_date.succ_opt().unwrap_or(start_date),
            };
            EventSchedule::AllDay {
                start: start_date,
                end: end_date,
            }
        }
    };

    Ok(CalendarEvent {
        uid,
        summary,
        description,
        location,
        schedule,
    })
}

pub(crate) fn parse_ics(ics_content: &str) -> Result<Vec<CalendarEvent>> {
    if !ics_content.contains("BEGIN:VCALENDAR") {
        anyhow::bail!("Invalid iCalendar: missing BEGIN:VCALENDAR");
    }

    let calendar = Calendar::from_str(ics_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse iCalendar data: {e}"))?;

    let mut events = Vec::new();
    for component in &calendar.components {
        if let CalendarComponent::Event(ev) = component {
            events.push(parse_vevent(ev)?);
        }
    }
    Ok(events)
}

fn serialize_event_to_ics(
    uid: &str,
    summary: &str,
    description: Option<&str>,
    location: Option<&str>,
    schedule: &EventSchedule,
    sequence: Option<u32>,
) -> String {
    let mut event = Event::new();
    event.uid(uid).summary(summary);

    if let Some(seq) = sequence {
        event.add_property("SEQUENCE", seq.to_string());
    }

    match *schedule {
        EventSchedule::Timed { start, end } => {
            event.starts(start).ends(end);
        }
        EventSchedule::AllDay { start, end } => {
            event.starts(start).ends(end);
        }
    }

    if let Some(desc) = description {
        event.description(desc);
    }

    if let Some(loc) = location {
        event.location(loc);
    }

    Calendar::new().push(event.done()).done().to_string()
}

pub(crate) fn build_created_ics(uid: &str, payload: &CreateEventPayload) -> String {
    serialize_event_to_ics(
        uid,
        &payload.summary,
        payload.description.as_deref(),
        payload.location.as_deref(),
        &payload.schedule,
        None,
    )
}

pub(crate) fn build_updated_ics(
    raw_ics: &str,
    uid: &str,
    payload: UpdateEventPayload,
) -> Result<String> {
    let calendar = Calendar::from_str(raw_ics)
        .map_err(|e| anyhow::anyhow!("Échec du parsing ICS : {e}"))?;

    let (existing_res, current_seq) = calendar
        .components
        .iter()
        .find_map(|component| match component {
            CalendarComponent::Event(ev) if ev.get_uid() == Some(uid) => {
                let seq = ev
                    .property_value("SEQUENCE")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                Some((parse_vevent(ev), seq))
            }
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("Événement '{uid}' introuvable dans l'ICS"))?;

    let existing = existing_res?;

    let new_seq = current_seq + 1;
    let new_summary = payload.summary.unwrap_or(existing.summary);
    let new_description = payload.description.unwrap_or(existing.description);
    let new_location = payload.location.unwrap_or(existing.location);
    let new_schedule = payload.schedule.unwrap_or(existing.schedule);

    Ok(serialize_event_to_ics(
        uid,
        &new_summary,
        new_description.as_deref(),
        new_location.as_deref(),
        &new_schedule,
        Some(new_seq),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    const SAMPLE_ICS: &str = "\
BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:event-123\r\n\
SEQUENCE:2\r\n\
SUMMARY:Réunion initiale\r\n\
DESCRIPTION:Ancienne description\r\n\
DTSTART:20261115T090000Z\r\n\
DTEND:20261115T100000Z\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    #[test]
    fn test_build_created_ics() {
        let payload = CreateEventPayload {
            summary: "Nouveau RDV".into(),
            description: Some("Notes".into()),
            location: None,
            schedule: EventSchedule::all_day(NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()),
        };

        let ics = build_created_ics("uid-999", &payload);
        assert!(ics.contains("UID:uid-999"));
        assert!(ics.contains("SUMMARY:Nouveau RDV"));
        assert!(ics.contains("DESCRIPTION:Notes"));
    }

    #[test]
    fn test_incremente_sequence_et_met_a_jour_summary() {
        let payload = UpdateEventPayload {
            summary: Some("Réunion modifiée".into()),
            ..Default::default()
        };

        let result = build_updated_ics(SAMPLE_ICS, "event-123", payload).unwrap();

        assert!(result.contains("SEQUENCE:3"));
        assert!(result.contains("SUMMARY:Réunion modifiée"));
        assert!(result.contains("DESCRIPTION:Ancienne description"));
    }

    #[test]
    fn test_suppression_description_avec_some_none() {
        let payload = UpdateEventPayload {
            description: Some(None),
            ..Default::default()
        };

        let result = build_updated_ics(SAMPLE_ICS, "event-123", payload).unwrap();
        assert!(!result.contains("DESCRIPTION:"));
    }

    #[test]
    fn test_erreur_si_uid_inexistant() {
        let payload = UpdateEventPayload::default();
        let result = build_updated_ics(SAMPLE_ICS, "uid-inconnu", payload);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ics_timed_event() {
        let ics = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//Example Corp.//EN\r\n\
BEGIN:VEVENT\r\n\
UID:event-123\r\n\
SUMMARY:Réunion Projet\r\n\
DESCRIPTION:Discussion technique sur CalDAV\r\n\
LOCATION:Salle Virtuelle\r\n\
DTSTART:20261110T090000Z\r\n\
DTEND:20261110T100000Z\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

        let result = parse_ics(ics);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.uid, "event-123");
        assert_eq!(event.summary, "Réunion Projet");
        assert_eq!(
            event.description.as_deref(),
            Some("Discussion technique sur CalDAV")
        );
        assert_eq!(event.location.as_deref(), Some("Salle Virtuelle"));

        let expected_start = Utc.with_ymd_and_hms(2026, 11, 10, 9, 0, 0).unwrap();
        let expected_end = Utc.with_ymd_and_hms(2026, 11, 10, 10, 0, 0).unwrap();
        assert_eq!(
            event.schedule,
            EventSchedule::Timed {
                start: expected_start,
                end: expected_end,
            }
        );
    }

    #[test]
    fn test_parse_ics_with_timezone() {
        let ics = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:tz-event-1\r\n\
SUMMARY:Point Paris\r\n\
DTSTART;TZID=Europe/Paris:20261110T100000\r\n\
DTEND;TZID=Europe/Paris:20261110T110000\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

        let result = parse_ics(ics);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.uid, "tz-event-1");

        // En novembre, Paris est en UTC+1 (heure d'hiver standard CET)
        // 10:00 Paris -> 09:00 UTC
        // 11:00 Paris -> 10:00 UTC
        let expected_start = Utc.with_ymd_and_hms(2026, 11, 10, 9, 0, 0).unwrap();
        let expected_end = Utc.with_ymd_and_hms(2026, 11, 10, 10, 0, 0).unwrap();
        assert_eq!(
            event.schedule,
            EventSchedule::Timed {
                start: expected_start,
                end: expected_end,
            }
        );
    }

    #[test]
    fn test_parse_ics_without_dtend() {
        // RFC 5545: si DTEND est absent, la fin vaut le début pour un timed event
        let ics_timed = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:no-end-1\r\n\
SUMMARY:Événement instantané\r\n\
DTSTART:20261110T140000Z\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

        let events = parse_ics(ics_timed).expect("should parse timed event without DTEND");
        assert_eq!(events.len(), 1);
        let start_utc = Utc.with_ymd_and_hms(2026, 11, 10, 14, 0, 0).unwrap();
        assert_eq!(
            events[0].schedule,
            EventSchedule::Timed {
                start: start_utc,
                end: start_utc,
            }
        );

        // RFC 5545: si DTEND est absent pour un all-day event, la fin vaut start + 1 jour
        let ics_allday = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:no-end-2\r\n\
SUMMARY:Journée sans DTEND\r\n\
DTSTART;VALUE=DATE:20261110\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

        let events = parse_ics(ics_allday).expect("should parse all-day event without DTEND");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].schedule,
            EventSchedule::AllDay {
                start: NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(),
                end: NaiveDate::from_ymd_opt(2026, 11, 11).unwrap(),
            }
        );
    }

    #[test]
    fn test_parse_ics_all_day_event() {
        let ics = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//Example Corp.//EN\r\n\
BEGIN:VEVENT\r\n\
UID:allday-456\r\n\
SUMMARY:Jour Férié\r\n\
DTSTART;VALUE=DATE:20261111\r\n\
DTEND;VALUE=DATE:20261112\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

        let result = parse_ics(ics);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.uid, "allday-456");
        assert_eq!(event.summary, "Jour Férié");
        assert_eq!(event.description, None);
        assert_eq!(event.location, None);

        let expected_start = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
        let expected_end = NaiveDate::from_ymd_opt(2026, 11, 12).unwrap();
        assert_eq!(
            event.schedule,
            EventSchedule::AllDay {
                start: expected_start,
                end: expected_end,
            }
        );
    }

    #[test]
    fn test_parse_ics_errors() {
        // Cas 1: VEVENT sans UID
        let ics_missing_uid = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
SUMMARY:Sans UID\r\n\
DTSTART:20261110T090000Z\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";
        let res = parse_ics(ics_missing_uid);
        assert!(res.is_err(), "devrait échouer car UID est manquant");

        // Cas 2: VEVENT sans DTSTART
        let ics_missing_dtstart = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:no-dtstart\r\n\
SUMMARY:Sans DTSTART\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";
        let res = parse_ics(ics_missing_dtstart);
        assert!(res.is_err(), "devrait échouer car DTSTART est manquant");

        // Cas 3: Contenu invalide
        let res = parse_ics("pas du tout un icalendar");
        assert!(res.is_err(), "devrait échouer sur du contenu invalide");
    }
}