//! Background sync worker — Tokio task that auto-syncs every 5 min.

use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Duration;

use crate::db::Database;
use crate::sync::google::{gcal_to_local, gtask_to_local, GoogleCalendarClient, GoogleConfig};

// ─── Channel types ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum SyncCommand {
    SyncNow,
    PushDirty,
    /// Exchange an OAuth code received from the browser callback for tokens.
    ExchangeCode(String),
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum SyncEvent {
    SyncStarted,
    SyncComplete { pulled: usize, pushed: usize },
    SyncError(String),
    /// No token found — the TUI should prompt the user to connect.
    AuthRequired,
    /// Token exchange succeeded — the TUI can show the connected state.
    AuthComplete,
}

// ─── Worker handle ────────────────────────────────────────────────────────────

pub struct SyncWorker {
    pub cmd_tx:   mpsc::Sender<SyncCommand>,
    pub event_rx: Arc<Mutex<mpsc::Receiver<SyncEvent>>>,
}

impl SyncWorker {
    /// Spawn the background worker. Always creates a Google client (credentials
    /// are embedded at compile time). `google_config` controls which
    /// calendar/task-list IDs to sync; falls back to "primary" / "@default".
    pub fn spawn(db: Database, google_config: GoogleConfig) -> Self {
        let (cmd_tx,   mut cmd_rx)   = mpsc::channel::<SyncCommand>(32);
        let (event_tx,     event_rx) = mpsc::channel::<SyncEvent>(64);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            interval.tick().await; // discard first immediate tick

            let client = Arc::new(Mutex::new(
                GoogleCalendarClient::new(google_config, db.clone())
            ));

            loop {
                tokio::select! {
                    cmd = cmd_rx.recv() => match cmd {
                        Some(SyncCommand::Shutdown) | None => break,

                        Some(SyncCommand::ExchangeCode(code)) => {
                            let mut c = client.lock().await;
                            match c.exchange_code(&code).await {
                                Ok(_) => {
                                    let _ = event_tx.send(SyncEvent::AuthComplete).await;
                                    tracing::info!("OAuth exchange succeeded, triggering sync");
                                    drop(c);
                                    run_sync(client.clone(), &db, &event_tx).await;
                                }
                                Err(e) => {
                                    tracing::error!("OAuth exchange failed: {e}");
                                    let _ = event_tx.send(
                                        SyncEvent::SyncError(format!("Auth failed: {e}"))
                                    ).await;
                                }
                            }
                        }

                        Some(SyncCommand::SyncNow) => {
                            if !check_auth(client.clone(), &event_tx).await { continue; }
                            run_sync(client.clone(), &db, &event_tx).await;
                        }

                        Some(SyncCommand::PushDirty) => {
                            if !check_auth(client.clone(), &event_tx).await { continue; }
                            push_dirty_events(client.clone(), &db, &event_tx).await;
                            push_dirty_tasks(client.clone(), &db, &event_tx).await;
                        }
                    },
                    _ = interval.tick() => {
                        if !check_auth(client.clone(), &event_tx).await { continue; }
                        run_sync(client.clone(), &db, &event_tx).await;
                    }
                }
            }

            tracing::info!("Sync worker stopped");
        });

        SyncWorker { cmd_tx, event_rx: Arc::new(Mutex::new(event_rx)) }
    }

    pub async fn sync_now(&self)    { let _ = self.cmd_tx.send(SyncCommand::SyncNow).await; }
    pub async fn push_dirty(&self)  { let _ = self.cmd_tx.send(SyncCommand::PushDirty).await; }
    pub async fn shutdown(&self)    { let _ = self.cmd_tx.send(SyncCommand::Shutdown).await; }

    pub async fn exchange_code(&self, code: String) {
        let _ = self.cmd_tx.send(SyncCommand::ExchangeCode(code)).await;
    }
}

// ─── Auth check helper ────────────────────────────────────────────────────────

/// Returns true if authenticated, false (and emits AuthRequired) if not.
async fn check_auth(
    client: Arc<Mutex<GoogleCalendarClient>>,
    tx:     &mpsc::Sender<SyncEvent>,
) -> bool {
    let mut c = client.lock().await;
    if c.ensure_authenticated().await.is_ok() {
        true
    } else {
        let _ = tx.send(SyncEvent::AuthRequired).await;
        false
    }
}

// ─── Full sync ────────────────────────────────────────────────────────────────

async fn run_sync(
    client: Arc<Mutex<GoogleCalendarClient>>,
    db:     &Database,
    tx:     &mpsc::Sender<SyncEvent>,
) {
    let _ = tx.send(SyncEvent::SyncStarted).await;
    tracing::info!("Full sync started");

    let mut pulled = 0usize;

    // ── Pull calendar events ──────────────────────────────────────────────────
    let cal_ids = {
        let c = client.lock().await;
        c.config.calendar_ids.clone()
    };

    for cal_id in &cal_ids {
        let events = {
            let mut c = client.lock().await;
            match c.pull_events(cal_id).await {
                Ok(evs) => evs,
                Err(e)  => {
                    tracing::warn!("pull_events({cal_id}): {e}");
                    let _ = tx.send(SyncEvent::SyncError(e.to_string())).await;
                    continue;
                }
            }
        };

        for ge in &events {
            if let Some(local) = gcal_to_local(ge, cal_id) {
                if db.upsert_remote_event(&local).await.is_ok() { pulled += 1; }
            }
        }
    }

    // ── Pull Google Tasks ─────────────────────────────────────────────────────
    let task_list_ids = {
        let c = client.lock().await;
        c.config.task_list_ids.clone()
    };

    for tl_id in &task_list_ids {
        let tasks = {
            let mut c = client.lock().await;
            match c.pull_tasks(tl_id).await {
                Ok(ts) => ts,
                Err(e) => {
                    tracing::warn!("pull_tasks({tl_id}): {e}");
                    let _ = tx.send(SyncEvent::SyncError(e.to_string())).await;
                    continue;
                }
            }
        };

        for gt in &tasks {
            if let Some(local) = gtask_to_local(gt, tl_id) {
                if db.upsert_remote_task(&local).await.is_ok() { pulled += 1; }
            }
        }
    }

    // ── Push dirty local changes ──────────────────────────────────────────────
    let pushed_ev = push_dirty_events(client.clone(), db, tx).await;
    let pushed_tk = push_dirty_tasks(client, db, tx).await;

    let pushed = pushed_ev + pushed_tk;
    let _ = tx.send(SyncEvent::SyncComplete { pulled, pushed }).await;
    tracing::info!("Sync done: pulled={pulled} pushed={pushed}");
}

// ─── Push dirty calendar events ───────────────────────────────────────────────

async fn push_dirty_events(
    client: Arc<Mutex<GoogleCalendarClient>>,
    db:     &Database,
    tx:     &mpsc::Sender<SyncEvent>,
) -> usize {
    let dirty = match db.dirty_events().await {
        Ok(v)  => v,
        Err(e) => { tracing::error!("dirty_events: {e}"); return 0; }
    };

    let mut pushed = 0usize;

    for ev in &dirty {
        let cal_id = ev.calendar_id.as_deref().unwrap_or("primary");
        let mut c  = client.lock().await;

        let result = if ev.deleted {
            if let Some(sid) = &ev.sync_id {
                c.delete_event(cal_id, sid).await.map(|_| (None, None))
            } else { Ok((None, None)) }
        } else if let Some(sid) = &ev.sync_id {
            c.update_event(cal_id, sid, ev).await.map(|etag| (None, Some(etag)))
        } else {
            c.push_event(cal_id, ev).await.map(|(id, etag)| (Some(id), Some(etag)))
        };

        match result {
            Ok((sid, etag)) => {
                if db.mark_event_clean(&ev.id, sid.as_deref(), etag.as_deref()).await.is_ok() {
                    pushed += 1;
                }
            }
            Err(e) => {
                tracing::warn!("push event failed for {}: {e}", ev.id);
                let _ = tx.send(SyncEvent::SyncError(e.to_string())).await;
            }
        }
    }
    pushed
}

// ─── Push dirty tasks ─────────────────────────────────────────────────────────

async fn push_dirty_tasks(
    client: Arc<Mutex<GoogleCalendarClient>>,
    db:     &Database,
    tx:     &mpsc::Sender<SyncEvent>,
) -> usize {
    let dirty = match db.dirty_tasks().await {
        Ok(v)  => v,
        Err(e) => { tracing::error!("dirty_tasks: {e}"); return 0; }
    };

    let mut pushed = 0usize;

    for task in &dirty {
        let tl_id = task.task_list_id.as_deref().unwrap_or("@default");
        let mut c = client.lock().await;

        let result = if task.deleted {
            if let Some(sid) = &task.sync_id {
                c.delete_task(tl_id, sid).await.map(|_| None)
            } else { Ok(None) }
        } else if let Some(sid) = &task.sync_id {
            c.update_task(tl_id, sid, task).await.map(|_| None)
        } else {
            c.push_task(tl_id, task).await.map(|(id, _)| Some(id))
        };

        match result {
            Ok(sid) => {
                if db.mark_task_clean(&task.id, sid.as_deref()).await.is_ok() {
                    pushed += 1;
                }
            }
            Err(e) => {
                tracing::warn!("push task failed for {}: {e}", task.id);
                let _ = tx.send(SyncEvent::SyncError(e.to_string())).await;
            }
        }
    }
    pushed
}
