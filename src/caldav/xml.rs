use anyhow::Result;
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

use super::calendar::event::parse_ics;
use super::models::CalendarEvent;

fn extract_calendar_data(xml: &str) -> Result<Vec<String>> {
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

pub(crate) fn parse_multistatus(xml: &str) -> Result<Vec<CalendarEvent>> {
    let raw_ics_list = extract_calendar_data(xml)?;
    let mut all_events = Vec::new();
    for ics in raw_ics_list {
        let events = parse_ics(&ics)?;
        all_events.extend(events);
    }
    Ok(all_events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_parsing() {
        let xml_data = r#"
        <multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
    <response>
        <propstat>
            <prop>
                <C:calendar-data>BEGIN:VCALENDAR
UID:test-1
SUMMARY:Premier événement
END:VCALENDAR</C:calendar-data>
               </prop>
           </propstat>
       </response>
       <response>
           <propstat>
               <prop>
                   <C:calendar-data>BEGIN:VCALENDAR
UID:test-2
SUMMARY:Deuxième événement
END:VCALENDAR</C:calendar-data>
               </prop>
           </propstat>
       </response>
   </multistatus>
        "#;

        let result = extract_calendar_data(xml_data);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            "BEGIN:VCALENDAR\nUID:test-1\nSUMMARY:Premier événement\nEND:VCALENDAR"
        );
        assert_eq!(
            events[1],
            "BEGIN:VCALENDAR\nUID:test-2\nSUMMARY:Deuxième événement\nEND:VCALENDAR"
        );
    }

    #[test]
    fn test_xml_parsing_with_cdata() {
        let xml_data = r#"<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
    <response>
        <propstat>
            <prop>
                <C:calendar-data><![CDATA[BEGIN:VCALENDAR
UID:cdata-test
SUMMARY:Evenement CDATA
END:VCALENDAR]]></C:calendar-data>
            </prop>
        </propstat>
    </response>
</multistatus>"#;

        let result = extract_calendar_data(xml_data);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            "BEGIN:VCALENDAR\nUID:cdata-test\nSUMMARY:Evenement CDATA\nEND:VCALENDAR"
        );
    }

    #[test]
    fn test_parse_multistatus_success() {
        let xml = r#"<?xml version="1.0" encoding="utf-8" ?>
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
UID:event-1
SUMMARY:Event One
DTSTART:20261110T080000Z
DTEND:20261110T090000Z
END:VEVENT
END:VCALENDAR</C:calendar-data>
            </D:prop>
        </D:propstat>
    </D:response>
</D:multistatus>"#;

        let result = parse_multistatus(xml);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].uid, "event-1");
        assert_eq!(events[0].summary, "Event One");
    }

    #[test]
    fn test_parse_multistatus_empty() {
        let xml = r#"<?xml version="1.0" encoding="utf-8" ?>
<D:multistatus xmlns:D="DAV:">
</D:multistatus>"#;

        let result = parse_multistatus(xml);
        assert!(result.is_ok());
        let events = result.unwrap();
        assert!(events.is_empty());
    }
}
