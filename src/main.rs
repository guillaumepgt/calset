use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use icalendar::{Calendar, Component, Event, EventLike};
use reqwest::{header::CONTENT_TYPE, Client, Method, StatusCode};
use uuid::Uuid;

struct CalDavClient {
    http: Client,
    base_url: String,
    user: String,
    pass: String,
    calendar_name: String,
}

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

    fn calendar_url(&self) -> String {
        format!("{}/{}/{}/", self.base_url, self.user, self.calendar_name)
    }

    /// Initialise le calendrier via MKCALENDAR (idempotent au démarrage du bot)
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
            StatusCode::CREATED => println!("Calendrier '{}' créé avec succès.", self.calendar_name),
            // Si le calendrier existe déjà, Radicale renvoie généralement 405 Method Not Allowed ou 409 Conflict
            StatusCode::METHOD_NOT_ALLOWED | StatusCode::CONFLICT | StatusCode::OK => {
                println!("Le calendrier '{}' existe déjà.", self.calendar_name);
            }
            status => {
                let body = response.text().await.unwrap_or_default();
                anyhow::bail!("Erreur lors de MKCALENDAR ({}): {}", status, body);
            }
        }

        Ok(())
    }

    /// Crée et envoie un événement .ics vers Radicale via PUT
    pub async fn push_event(
        &self,
        summary: &str,
        description: &str,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<String> {
        let event_uid = Uuid::new_v4().to_string();

        // Construction du payload iCalendar RFC 5545
        let event = Event::new()
            .uid(&event_uid)
            .summary(summary)
            .description(description)
            .starts(start)
            .ends(end)
            .done();

        let calendar = Calendar::new().push(event).done();
        let ics_payload = calendar.to_string();

        // URL cible : http://127.0.0.1:5232/mon_utilisateur/agenda/<UID>.ics
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

#[tokio::main]
async fn main() -> Result<()> {
    let client = CalDavClient::new(
        "http://127.0.0.1:5232",
        "mon_utilisateur",
        "mon_mot_de_passe_robuste",
        "agenda",
    );

    // 1. S'assurer que l'agenda existe au lancement du microservice
    client.ensure_calendar_exists().await?;

    // 2. Créer un événement de test (dans 1 heure, durée 45 min)
    let start = Utc::now() + Duration::hours(1);
    let end = start + Duration::minutes(45);

    let uid = client
        .push_event(
            "Appeler le comptable",
            "Tâche reçue depuis Telegram",
            start,
            end,
        )
        .await?;

    println!("Événement synchronisé avec l'UID : {}", uid);

    Ok(())
}