use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::{header::CONTENT_TYPE, Client, Method, StatusCode};
use tracing::info;
use uuid::Uuid;

use super::calendar::event::{build_created_ics, build_updated_ics};
use super::models::{CalDavClient, CalendarEvent, CreateEventPayload, UpdateEventPayload};
use super::xml::parse_multistatus;
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
            StatusCode::CREATED => {
                info!("Calendar '{}' created successfully.", self.calendar_name)
            }
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::CONFLICT | StatusCode::OK => {
                info!("The calendar '{}' already exists.", self.calendar_name)
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                anyhow::bail!(
                    "CalDAV authentication failed ({}) : check CAL_USER and CAL_PASS",
                    response.status()
                )
            }
            status => {
                let body = response.text().await.unwrap_or_default();
                anyhow::bail!("Failed to create calendar ({}): {}", status, body);
            }
        }
        Ok(())
    }

    pub async fn create_event(&self, payload: CreateEventPayload) -> Result<String> {
        let event_uid = Uuid::new_v4().to_string();
        let ics_payload = build_created_ics(&event_uid, &payload);

        self.put_ics(&event_uid, ics_payload)
            .await
            .context("Échec de création de l'événement")?;

        Ok(event_uid)
    }

    pub async fn update_event(&self, uid: &str, payload: UpdateEventPayload) -> Result<()> {
        let raw_ics = self
            .get_event_raw(uid)
            .await
            .context("Failed to retrieve the event for update")?;

        let ics_payload = build_updated_ics(&raw_ics, uid, payload)
            .context("Failed to build updated ICS payload")?;

        self.put_ics(uid, ics_payload)
            .await
            .context("Failed to update event")
    }

    async fn put_ics(&self, uid: &str, ics_payload: String) -> Result<()> {
        let event_url = format!("{}{}.ics", self.calendar_url(), uid);

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
            anyhow::bail!("Erreur HTTP PUT ({}): {}", status, body);
        }

        Ok(())
    }

    pub async fn list_events(
        &self,
        start_utc: DateTime<Utc>,
        end_utc: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>> {
        if start_utc >= end_utc {
            anyhow::bail!("The start date must be earlier than the end date");
        }

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

        let method = Method::from_bytes(b"REPORT").context("Invalid HTTP method REPORT")?;

        let response = self
            .http
            .request(method, self.calendar_url())
            .basic_auth(&self.user, Some(&self.pass))
            .header(CONTENT_TYPE, "application/xml; charset=utf-8")
            .header("Depth", "1")
            .body(calendar_query)
            .send()
            .await
            .context("Failed to send calendar query (REPORT)")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list events ({}): {}", status, body);
        }
        let body = response
            .text()
            .await
            .context("Failed to read response body for list events")?;
        parse_multistatus(&body)
    }

    pub async fn get_event_raw(&self, uid: &str) -> Result<String> {
        let event_url = format!("{}{}.ics", self.calendar_url(), uid);

        let response = self
            .http
            .get(&event_url)
            .basic_auth(&self.user, Some(&self.pass))
            .send()
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            anyhow::bail!("Event not found with this UID '{}' (404)", uid);
        }

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to retrieve the event '{}' : {}",
                uid,
                response.status()
            );
        }
        Ok(response.text().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[tokio::test]
    async fn test_list_events_invalid_range() {
        let client = CalDavClient::new("http://localhost", "user", "pass", "calendar");

        let start_utc = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let end_utc = Utc.with_ymd_and_hms(2026, 1, 31, 23, 59, 59).unwrap();

        let result = client.list_events(start_utc, end_utc).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "The start date must be earlier than the end date"
        );
    }
}