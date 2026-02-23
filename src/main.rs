mod app;
mod calendar;
mod config;
mod db;
mod holidays;
mod sync;
mod tasks;
mod theme;
mod ui;

use anyhow::Result;
use app::{App, AuthState};
use config::AppConfig;
use db::Database;
use sync::worker::SyncWorker;
use theme::ThemeConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    run_tui().await
}

// ─── TUI ─────────────────────────────────────────────────────────────────────

async fn run_tui() -> Result<()> {
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("lifemanager");
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = tracing_appender::rolling::daily(&log_dir, "lifemanager.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
        .init();

    tracing::info!("Starting LifeManager");

    let cfg   = AppConfig::load().unwrap_or_default();
    let theme = ThemeConfig::load()?;
    let db    = Database::connect().await?;
    db.migrate().await?;

    // Use calendar/task-list IDs from config, or defaults.
    // Credentials are embedded at compile time — users never configure them.
    let google_config = cfg.google.unwrap_or_default();
    let worker = SyncWorker::spawn(db.clone(), google_config);

    let mut app = App::new(db, theme).await?;
    app.attach_sync_worker(worker);

    // Already authenticated from a previous session — kick off a sync right away.
    if app.auth_state == AuthState::Connected {
        if let Some(ref w) = app.sync {
            w.sync_now().await;
        }
    }

    app.run().await?;
    Ok(())
}
