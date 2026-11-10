use crate::caldav::models::{CalendarEvent, EventSchedule};
use anyhow::Context;
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use icalendar::{Calendar, CalendarComponent, Component, DatePerhapsTime, EventLike};
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::str::FromStr;

pub fn extract_calendar_data(xml: &str) -> anyhow::Result<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    let mut results = Vec::new();
    let mut in_calendar_data = false;
    let mut current_data = String::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => {
                if e.local_name().as_ref() == "calendar-data" {
                    in_calendar_data = true;
                    current_data.clear();
                }
            }
            Event::Text(e) => {
                if in_calendar_data {
                    current_data.push_str(&unescape(&e)?);
                }
            }
            Event::CData(e) => {
                if in_calendar_data {
                    current_data.push_str(e.as_ref());
                }
            }
            Event::End(e) => {
                if e.local_name().as_ref() == "calendar-data" {
                    in_calendar_data = false;
                    results.push(current_data.trim().to_string());
                }
            }
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }
    Ok(results)
}

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

pub fn parse_vevent(event: &icalendar::Event) -> anyhow::Result<CalendarEvent> {
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

pub fn parse_ics(ics_content: &str) -> anyhow::Result<Vec<CalendarEvent>> {
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

pub fn parse_multistatus(xml: &str) -> anyhow::Result<Vec<CalendarEvent>> {
    let raw_ics_list = extract_calendar_data(xml)?;
    let mut all_events = Vec::new();
    for ics in raw_ics_list {
        let events = parse_ics(&ics)?;
        all_events.extend(events);
    }
    Ok(all_events)
}
