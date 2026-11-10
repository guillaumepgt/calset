use calset::caldav::models::EventSchedule;
use calset::caldav::parser::{extract_calendar_data, parse_ics, parse_multistatus};
use chrono::{NaiveDate, TimeZone, Utc};

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
            end: expected_end
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
            end: expected_end
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
            end: start_utc
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
            end: expected_end
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
