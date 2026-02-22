mod app;
mod calendar;
mod config;
mod db;
mod holidays;
mod sync;
mod tasks;
mod theme;
mod ui;

use anyhow::{anyhow, Result};
use app::App;
use config::AppConfig;
use db::Database;
use sync::google::GoogleCalendarClient;
use sync::worker::SyncWorker;
use theme::ThemeConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // ── lm auth google ────────────────────────────────────────────────────────
    if args.get(1).map(|s| s.as_str()) == Some("auth")
        && args.get(2).map(|s| s.as_str()) == Some("google")
    {
        return cmd_auth_google().await;
    }

    // ── lm sync ───────────────────────────────────────────────────────────────
    if args.get(1).map(|s| s.as_str()) == Some("sync") {
        return cmd_sync().await;
    }

    // ── lm (TUI) ──────────────────────────────────────────────────────────────
    run_tui().await
}

// ─── Auth command ─────────────────────────────────────────────────────────────

async fn cmd_auth_google() -> Result<()> {
    // Logging to stderr so it doesn't interfere with terminal output
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let cfg = AppConfig::load()?;
    let google = cfg.google.ok_or_else(|| {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("lifemanager")
            .join("config.toml");
        anyhow!(
            "No [google] section found in {}\n\
             Copy config.example.toml and fill in your client_id and client_secret.",
            config_path.display()
        )
    })?;

    let db = Database::connect().await?;
    db.migrate().await?;

    let mut client = GoogleCalendarClient::new(google, db);
    let url = client.build_auth_url();

    println!("\nOpening Google authorization in your browser…");
    println!("If it doesn't open automatically, visit:\n\n  {url}\n");

    // Try to open in browser; ignore errors (user can open manually)
    let _ = open::that(&url);

    println!("Waiting for Google to redirect back (listening on :8085)…");

    let code = GoogleCalendarClient::listen_for_callback().await?;
    client.exchange_code(&code).await?;

    println!("\nSuccess! Google Calendar and Tasks are now authorized.");
    println!("Run  lm  to start the app — it will sync automatically.");

    Ok(())
}

// ─── Manual sync command ──────────────────────────────────────────────────────

async fn cmd_sync() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    let cfg = AppConfig::load()?;
    if cfg.google.is_none() {
        println!("No [google] config found. Run  lm auth google  first.");
        return Ok(());
    }

    let db     = Database::connect().await?;
    db.migrate().await?;
    let worker = SyncWorker::spawn(db.clone(), cfg.google);
    worker.sync_now().await;

    // Give the worker time to complete before exiting
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    worker.shutdown().await;
    println!("Sync complete.");
    Ok(())
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

    let has_google = cfg.google.is_some();
    let worker     = SyncWorker::spawn(db.clone(), cfg.google);

    let mut app = App::new(db, theme).await?;
    app.attach_sync_worker(worker);

    if has_google {
        if let Some(ref w) = app.sync {
            w.sync_now().await;
        }
    }

    app.run().await?;
    Ok(())
}
