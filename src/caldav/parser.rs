use crate::caldav::models::CalendarEvent;

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use quick_xml::escape::unescape;

pub(crate) fn extract_calendar_data(xml: &str) -> anyhow::Result<Vec<String>> {
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
            },
            Event::Text(e) => {
                if in_calendar_data {
                    current_data.push_str(&unescape(&e)?);
                }
            },
            Event::CData(e) => {
                if in_calendar_data {
                    current_data.push_str(&e);
                }
            },
            Event::End(e) => {
                if e.local_name().as_ref() == "calendar-data" {
                    in_calendar_data = false;
                    results.push(current_data.trim().to_string());
                }
            },
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }
    Ok(results)
}