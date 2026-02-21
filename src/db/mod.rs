use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Row};
use std::path::PathBuf;
use uuid::Uuid;

// ─── Domain models ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub all_day: bool,
    pub calendar_id: Option<String>,
    pub sync_id: Option<String>,
    pub etag: Option<String>,
    pub dirty: bool,
    pub deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Event {
    pub fn new(title: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(), title: title.to_owned(),
            description: None, start, end, all_day: false,
            calendar_id: None, sync_id: None, etag: None,
            dirty: true, deleted: false, created_at: now, updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub notes: Option<String>,
    pub due: Option<DateTime<Utc>>,
    pub completed: bool,
    pub priority: i64,
    pub task_list_id: Option<String>,
    pub sync_id: Option<String>,
    pub dirty: bool,
    pub deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(title: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(), title: title.to_owned(),
            notes: None, due: None, completed: false, priority: 0,
            task_list_id: None, sync_id: None,
            dirty: true, deleted: false, created_at: now, updated_at: now,
        }
    }
}

// ─── Database ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn connect() -> Result<Self> {
        let db_path = data_dir().join("lifemanager.db");
        std::fs::create_dir_all(db_path.parent().unwrap())?;
        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        Ok(Self { pool: SqlitePool::connect(&url).await? })
    }

    pub async fn migrate(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT,
                start TEXT NOT NULL, end TEXT NOT NULL, all_day INTEGER NOT NULL DEFAULT 0,
                calendar_id TEXT, sync_id TEXT, etag TEXT,
                dirty INTEGER NOT NULL DEFAULT 1, deleted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            )"
        ).execute(&self.pool).await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_start ON events(start)")
            .execute(&self.pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, notes TEXT, due TEXT,
                completed INTEGER NOT NULL DEFAULT 0, priority INTEGER NOT NULL DEFAULT 0,
                task_list_id TEXT, sync_id TEXT,
                dirty INTEGER NOT NULL DEFAULT 1, deleted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL, updated_at TEXT NOT NULL
            )"
        ).execute(&self.pool).await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due)")
            .execute(&self.pool).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS oauth_tokens (
                provider TEXT PRIMARY KEY, access_token TEXT NOT NULL,
                refresh_token TEXT, expires_at TEXT
            )"
        ).execute(&self.pool).await?;

        tracing::info!("DB migrations complete");
        Ok(())
    }

    // ── Events ────────────────────────────────────────────────────────────────

    pub async fn upsert_event(&self, e: &Event) -> Result<()> {
        sqlx::query(
            "INSERT INTO events
                (id,title,description,start,end,all_day,calendar_id,sync_id,etag,dirty,deleted,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)
             ON CONFLICT(id) DO UPDATE SET
                title=excluded.title, description=excluded.description,
                start=excluded.start, end=excluded.end, all_day=excluded.all_day,
                calendar_id=excluded.calendar_id, sync_id=excluded.sync_id,
                etag=excluded.etag, dirty=excluded.dirty, deleted=excluded.deleted,
                updated_at=excluded.updated_at"
        )
        .bind(&e.id).bind(&e.title).bind(&e.description)
        .bind(e.start.to_rfc3339()).bind(e.end.to_rfc3339())
        .bind(e.all_day as i32).bind(&e.calendar_id)
        .bind(&e.sync_id).bind(&e.etag)
        .bind(e.dirty as i32).bind(e.deleted as i32)
        .bind(e.created_at.to_rfc3339()).bind(e.updated_at.to_rfc3339())
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn events_in_range(&self, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<Event>> {
        let rows = sqlx::query(
            "SELECT * FROM events WHERE start >= ? AND start < ? AND deleted=0 ORDER BY start"
        )
        .bind(from.to_rfc3339()).bind(to.to_rfc3339())
        .fetch_all(&self.pool).await?;
        rows.iter().map(row_to_event).collect()
    }

    pub async fn dirty_events(&self) -> Result<Vec<Event>> {
        let rows = sqlx::query("SELECT * FROM events WHERE dirty=1")
            .fetch_all(&self.pool).await?;
        rows.iter().map(row_to_event).collect()
    }

    /// Upsert an event that came from a remote (Google Calendar) pull.
    /// Deduplicates by sync_id and preserves locally-dirty events.
    pub async fn upsert_remote_event(&self, e: &Event) -> Result<()> {
        if let Some(sid) = &e.sync_id {
            if let Some(row) = sqlx::query("SELECT id, dirty FROM events WHERE sync_id=?")
                .bind(sid).fetch_optional(&self.pool).await?
            {
                let local_id: String = row.get("id");
                let dirty: i32       = row.get("dirty");
                if dirty != 0 {
                    return Ok(()); // user has local changes — don't overwrite
                }
                let mut updated = e.clone();
                updated.id    = local_id;
                updated.dirty = false;
                return self.upsert_event(&updated).await;
            }
        }
        let mut new_e = e.clone();
        new_e.dirty = false;
        self.upsert_event(&new_e).await
    }

    pub async fn mark_event_clean(&self, id: &str, sync_id: Option<&str>, etag: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE events SET dirty=0, sync_id=COALESCE(?,sync_id), etag=COALESCE(?,etag) WHERE id=?"
        )
        .bind(sync_id).bind(etag).bind(id)
        .execute(&self.pool).await?;
        Ok(())
    }

    // ── Tasks ─────────────────────────────────────────────────────────────────

    pub async fn upsert_task(&self, t: &Task) -> Result<()> {
        sqlx::query(
            "INSERT INTO tasks
                (id,title,notes,due,completed,priority,task_list_id,sync_id,dirty,deleted,created_at,updated_at)
             VALUES (?,?,?,?,?,?,?,?,?,?,?,?)
             ON CONFLICT(id) DO UPDATE SET
                title=excluded.title, notes=excluded.notes, due=excluded.due,
                completed=excluded.completed, priority=excluded.priority,
                task_list_id=excluded.task_list_id, sync_id=excluded.sync_id,
                dirty=excluded.dirty, deleted=excluded.deleted, updated_at=excluded.updated_at"
        )
        .bind(&t.id).bind(&t.title).bind(&t.notes)
        .bind(t.due.as_ref().map(|d| d.to_rfc3339()))
        .bind(t.completed as i32).bind(t.priority).bind(&t.task_list_id)
        .bind(&t.sync_id).bind(t.dirty as i32).bind(t.deleted as i32)
        .bind(t.created_at.to_rfc3339()).bind(t.updated_at.to_rfc3339())
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn dirty_tasks(&self) -> Result<Vec<Task>> {
        let rows = sqlx::query("SELECT * FROM tasks WHERE dirty=1")
            .fetch_all(&self.pool).await?;
        rows.iter().map(row_to_task).collect()
    }

    pub async fn mark_task_clean(&self, id: &str, sync_id: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE tasks SET dirty=0, sync_id=COALESCE(?,sync_id) WHERE id=?"
        )
        .bind(sync_id).bind(id)
        .execute(&self.pool).await?;
        Ok(())
    }

    /// Upsert a task that came from a remote (Google Tasks) pull.
    /// Deduplicates by sync_id and preserves locally-dirty tasks.
    pub async fn upsert_remote_task(&self, t: &Task) -> Result<()> {
        if let Some(sid) = &t.sync_id {
            if let Some(row) = sqlx::query("SELECT id, dirty FROM tasks WHERE sync_id=?")
                .bind(sid).fetch_optional(&self.pool).await?
            {
                let local_id: String = row.get("id");
                let dirty: i32       = row.get("dirty");
                if dirty != 0 {
                    return Ok(()); // user has local changes — don't overwrite
                }
                let mut updated = t.clone();
                updated.id    = local_id;
                updated.dirty = false;
                return self.upsert_task(&updated).await;
            }
        }
        let mut new_t = t.clone();
        new_t.dirty = false;
        self.upsert_task(&new_t).await
    }

    pub async fn all_tasks(&self) -> Result<Vec<Task>> {
        let rows = sqlx::query(
            "SELECT * FROM tasks WHERE deleted=0 ORDER BY priority DESC, due, title"
        ).fetch_all(&self.pool).await?;
        rows.iter().map(row_to_task).collect()
    }

    // ── OAuth tokens ──────────────────────────────────────────────────────────

    pub async fn save_token(
        &self, provider: &str, access: &str,
        refresh: Option<&str>, expires: Option<DateTime<Utc>>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO oauth_tokens (provider,access_token,refresh_token,expires_at)
             VALUES (?,?,?,?)
             ON CONFLICT(provider) DO UPDATE SET
                access_token=excluded.access_token,
                refresh_token=COALESCE(excluded.refresh_token,refresh_token),
                expires_at=excluded.expires_at"
        )
        .bind(provider).bind(access).bind(refresh)
        .bind(expires.as_ref().map(|e| e.to_rfc3339()))
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_token(
        &self, provider: &str,
    ) -> Result<Option<(String, Option<String>, Option<DateTime<Utc>>)>> {
        let row = sqlx::query(
            "SELECT access_token, refresh_token, expires_at FROM oauth_tokens WHERE provider=?"
        )
        .bind(provider).fetch_optional(&self.pool).await?;

        Ok(row.map(|r| {
            let access: String         = r.get("access_token");
            let refresh: Option<String> = r.get("refresh_token");
            let exp_s: Option<String>   = r.get("expires_at");
            let exp = exp_s.and_then(|s|
                DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))
            );
            (access, refresh, exp)
        }))
    }
}

// ─── Row helpers ─────────────────────────────────────────────────────────────

fn row_to_event(row: &sqlx::sqlite::SqliteRow) -> Result<Event> {
    Ok(Event {
        id:          row.get("id"),
        title:       row.get("title"),
        description: row.get("description"),
        start:       parse_dt(row.get("start"))?,
        end:         parse_dt(row.get("end"))?,
        all_day:     row.get::<i32, _>("all_day") != 0,
        calendar_id: row.get("calendar_id"),
        sync_id:     row.get("sync_id"),
        etag:        row.get("etag"),
        dirty:       row.get::<i32, _>("dirty") != 0,
        deleted:     row.get::<i32, _>("deleted") != 0,
        created_at:  parse_dt(row.get("created_at"))?,
        updated_at:  parse_dt(row.get("updated_at"))?,
    })
}

fn row_to_task(row: &sqlx::sqlite::SqliteRow) -> Result<Task> {
    let due_s: Option<String> = row.get("due");
    Ok(Task {
        id:           row.get("id"),
        title:        row.get("title"),
        notes:        row.get("notes"),
        due:          due_s.and_then(|s| parse_dt(s).ok()),
        completed:    row.get::<i32, _>("completed") != 0,
        priority:     row.get("priority"),
        task_list_id: row.get("task_list_id"),
        sync_id:      row.get("sync_id"),
        dirty:        row.get::<i32, _>("dirty") != 0,
        deleted:      row.get::<i32, _>("deleted") != 0,
        created_at:   parse_dt(row.get("created_at"))?,
        updated_at:   parse_dt(row.get("updated_at"))?,
    })
}

fn parse_dt(s: String) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(&s)?.with_timezone(&Utc))
}

fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("lifemanager")
}
