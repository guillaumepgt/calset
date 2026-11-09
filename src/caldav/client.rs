use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use icalendar::{Calendar, Component, Event, EventLike};
use reqwest::{header::CONTENT_TYPE, Client, Method, StatusCode};
use tracing::info;
use uuid::Uuid;

use super::models::{CalDavClient, CreateEventPayload, EventSchedule};
use crate::config::AppConfig;

impl CalDavClient {
    pub fn new(base_url: &str, user: &str, pass: &str, calendar_name: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            user: user.to_string(),
            pass: pass.to_string(),
            calendar_name: calendar_name.trim_matches('/').to_string(),
        }
    }

    pub fn from_config(config: &AppConfig) -> Self {
        Self::new(
            &config.radicale_url,
            &config.cal_user,
            &config.cal_pass,
            &config.cal_name,
        )
    }

    fn calendar_url(&self) -> String {
        format!("{}/{}/{}/", self.base_url, self.user, self.calendar_name)
    }

    pub async fn ensure_calendar_exists(&self) -> Result<()> {
        let method = Method::from_bytes(b"MKCALENDAR")
            .context("Méthode HTTP MKCALENDAR invalide")?;

        let response = self
            .http
            .request(method, self.calendar_url())
            .basic_auth(&self.user, Some(&self.pass))
            .send()
            .await
            .context("Échec de la connexion à Radicale")?;

        match response.status() {
            StatusCode::CREATED =>
                info!("Calendar '{}' created successfully.", self.calendar_name),

            StatusCode::METHOD_NOT_ALLOWED | StatusCode::CONFLICT | StatusCode::OK =>
                info!("The calendar '{}' already exists.", self.calendar_name),

            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN =>
                anyhow::bail!(
              "CalDAV authentication failed ({}) : check CAL_USER and CAL_PASS",
              response.status()),

            status => {
                let body = response.text().await.unwrap_or_default();
                anyhow::bail!("Failed to create calendar ({}): {}", status, body);
            }
        }
        Ok(())
    }

    pub async fn create_event(&self, payload: CreateEventPayload) -> Result<String> {
        let event_uid = Uuid::new_v4().to_string();

        let mut event = Event::new();
        event.uid(&event_uid)
            .summary(&payload.summary);

        match payload.schedule {
            EventSchedule::Timed { start, end } => {
                event.starts(start).ends(end);
            }
            EventSchedule::AllDay { start, end } => {
                event.starts(start).ends(end);
            }
        }

        if let Some(description) = payload.description {
            event.description(&description);
        }

        if let Some(location) = payload.location {
            event.location(&location);
        }

        let event = event.done();
        let calendar = Calendar::new().push(event).done();
        let ics_payload = calendar.to_string();

        let event_url = format!("{}{}.ics", self.calendar_url(), event_uid);

        let response = self
            .http
            .put(&event_url)
            .basic_auth(&self.user, Some(&self.pass))
            .header(CONTENT_TYPE, "text/calendar; charset=utf-8")
            .body(ics_payload)
            .send()
            .await
            .context("Échec de l'envoi de l'événement (PUT)")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Échec de création de l'événement ({}): {}", status, body);
        }

        Ok(event_uid)
    }

    pub async fn list_events(&self, start_utc: DateTime<Utc>, end_utc: DateTime<Utc>) -> Result<String> {
        if start_utc >= end_utc {anyhow::bail!("The start date must be earlier than the end date");}

        let start = start_utc.format("%Y%m%dT%H%M%SZ").to_string();
        let end = end_utc.format("%Y%m%dT%H%M%SZ").to_string();
        let calendar_query = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <c:calendar-query xmlns:c="urn:ietf:params:xml:ns:caldav">
                <d:prop xmlns:d="DAV:">
                    <d:getetag/>
                    <c:calendar-data/>
                </d:prop>
                <c:filter>
                    <c:comp-filter name="VCALENDAR">
                        <c:comp-filter name="VEVENT">
                            <c:time-range start="{start}" end="{end}"/>
                        </c:comp-filter>
                    </c:comp-filter>
                </c:filter>
            </c:calendar-query>"#
        );

        let method = Method::from_bytes(b"REPORT")
            .context("Invalid HTTP method REPORT")?;

        let response = self
            .http
            .request(method, self.calendar_url())
            .basic_auth(&self.user, Some(&self.pass))
            .header(CONTENT_TYPE, "application/xml; charset=utf-8")
            .header("Depth", "1")
            .body(calendar_query)
            .send().await.context("Failed to send calendar query (REPORT)")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list events ({}): {}", status, body);
        }
        Ok(response.text().await.context("Failed to read response body for list events")?)
    }
}