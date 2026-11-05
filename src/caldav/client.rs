use anyhow::{Context, Result};
use icalendar::{Calendar, Component, Event, EventLike};
use reqwest::{header::CONTENT_TYPE, Client, Method, StatusCode};
use tracing::info;
use uuid::Uuid;

use super::types::{CalDavClient, CreateEventPayload};
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

    pub async fn push_event(&self, payload: CreateEventPayload) -> Result<String> {
        let event_uid = Uuid::new_v4().to_string();

        let event = Event::new()
            .uid(&event_uid)
            .summary(&payload.summary)
            .description(&payload.description)
            .starts(payload.start)
            .ends(payload.end)
            .done();

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
}