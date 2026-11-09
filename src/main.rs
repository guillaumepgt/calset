use anyhow::Result;
use calset::caldav::{CalDavClient, CreateEventPayload, EventSchedule};
use calset::config;
use calset::telemetry;
use chrono::{Duration, Utc};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    telemetry::init_tracing();
    dotenvy::dotenv().ok();

    let cfg = config::load_from_env()?;
    info!("Démarrage avec l'utilisateur '{}' sur '{}'", cfg.cal_user, cfg.radicale_url);

    let caldav = CalDavClient::from_config(&cfg);
    caldav.ensure_calendar_exists().await?;

    let start = Utc::now() + Duration::hours(1);
    let end = start + Duration::minutes(45);

    let payload = CreateEventPayload {
        summary: "Appeler le comptable".to_string(),
        description: Some("Tâche issue de Telegram".to_string()),
        location: Some("Bureau".to_string()),
        schedule: EventSchedule::timed(start, end),
    };

    let uid = caldav.create_event(payload).await?;
    info!("Événement synchronisé (UID: {})", uid);

    Ok(())
}