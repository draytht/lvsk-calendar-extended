//! Google Calendar + Tasks OAuth2 & REST API client.
//!
//! Credentials are embedded at compile time:
//!   GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET must be set as env vars when
//!   building the release binary. Users never need to configure credentials.
//!
//! Auth flow (first run):
//!   1. build_auth_url() → open in browser
//!   2. listen_for_callback() → captures redirect with ?code=
//!   3. exchange_code(code) → stores tokens in DB
//!   4. All subsequent calls auto-refresh if expired

use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::db::{Database, Event, Task};

// ─── Compile-time credentials (set by the developer at build time) ────────────

const CLIENT_ID: &str = env!(
    "GOOGLE_CLIENT_ID",
    "Set GOOGLE_CLIENT_ID env var when building: GOOGLE_CLIENT_ID=xxx cargo build --release"
);
const CLIENT_SECRET: &str = env!(
    "GOOGLE_CLIENT_SECRET",
    "Set GOOGLE_CLIENT_SECRET env var when building: GOOGLE_CLIENT_SECRET=xxx cargo build --release"
);

const AUTH_URL:     &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL:    &str = "https://oauth2.googleapis.com/token";
const REDIRECT_URI: &str = "http://localhost:8085/callback";
const SCOPES:       &str = "https://www.googleapis.com/auth/calendar \
                             https://www.googleapis.com/auth/tasks";

// ─── Config (calendar/task IDs only — no credentials needed from users) ───────

fn default_calendar_ids() -> Vec<String> { vec!["primary".to_owned()] }
fn default_task_lists()    -> Vec<String> { vec!["@default".to_owned()] }

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleConfig {
    #[serde(default = "default_calendar_ids")]
    pub calendar_ids:  Vec<String>,
    #[serde(default = "default_task_lists")]
    pub task_list_ids: Vec<String>,
}

impl Default for GoogleConfig {
    fn default() -> Self {
        Self {
            calendar_ids:  default_calendar_ids(),
            task_list_ids: default_task_lists(),
        }
    }
}

// ─── Token response ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token:  String,
    refresh_token: Option<String>,
    expires_in:    Option<i64>,
}

// ─── Calendar API types ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GCalDateTime {
    pub date_time: Option<String>,
    pub date:      Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GCalEvent {
    pub id:          Option<String>,
    pub summary:     Option<String>,
    pub description: Option<String>,
    pub start:       Option<GCalDateTime>,
    pub end:         Option<GCalDateTime>,
    pub etag:        Option<String>,
    pub status:      Option<String>,
}

// ─── Tasks API types ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GTask {
    pub id:      Option<String>,
    pub title:   Option<String>,
    pub notes:   Option<String>,
    pub status:  Option<String>,
    pub due:     Option<String>,
    pub deleted: Option<bool>,
    pub hidden:  Option<bool>,
}

// ─── Client ───────────────────────────────────────────────────────────────────

pub struct GoogleCalendarClient {
    http:             Client,
    pub config:       GoogleConfig,
    db:               Database,
    access_token:     Option<String>,
    token_expires_at: Option<DateTime<Utc>>,
}

impl GoogleCalendarClient {
    pub fn new(config: GoogleConfig, db: Database) -> Self {
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("LifeManager/0.1")
                .build().expect("http client"),
            config, db,
            access_token: None,
            token_expires_at: None,
        }
    }

    // ── Auth flow ─────────────────────────────────────────────────────────────

    /// Build the Google OAuth authorization URL. No credentials needed from caller —
    /// CLIENT_ID is embedded at compile time.
    pub fn build_auth_url() -> String {
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&access_type=offline&prompt=consent",
            AUTH_URL,
            pct(CLIENT_ID),
            pct(REDIRECT_URI),
            pct(SCOPES),
        )
    }

    /// One-shot TCP listener on :8085 — blocks until Google redirects back.
    pub async fn listen_for_callback() -> Result<String> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:8085").await?;
        let (mut stream, _) = listener.accept().await?;
        let mut reader = BufReader::new(&mut stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        // GET /callback?code=XXX HTTP/1.1
        let code = line
            .split_whitespace().nth(1)
            .and_then(|path| path.splitn(2, '?').nth(1))
            .and_then(|qs| qs.split('&').find_map(|kv| {
                let mut p = kv.splitn(2, '=');
                if p.next()? == "code" { p.next().map(str::to_owned) } else { None }
            }))
            .ok_or_else(|| anyhow!("No code in OAuth callback"))?;

        stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
              <html><body><h2>Authorized! You can close this tab.</h2></body></html>"
        ).await?;

        Ok(code)
    }

    pub async fn exchange_code(&mut self, code: &str) -> Result<()> {
        let mut p = HashMap::new();
        p.insert("code",          code);
        p.insert("client_id",     CLIENT_ID);
        p.insert("client_secret", CLIENT_SECRET);
        p.insert("redirect_uri",  REDIRECT_URI);
        p.insert("grant_type",    "authorization_code");

        let resp: TokenResponse = self.http.post(TOKEN_URL).form(&p)
            .send().await?.error_for_status()?.json().await?;
        self.store_tokens(resp).await
    }

    // ── Token management ──────────────────────────────────────────────────────

    async fn store_tokens(&mut self, t: TokenResponse) -> Result<()> {
        let exp = t.expires_in.map(|s| Utc::now() + Duration::seconds(s - 60));
        self.db.save_token("google", &t.access_token, t.refresh_token.as_deref(), exp).await?;
        self.access_token     = Some(t.access_token);
        self.token_expires_at = exp;
        Ok(())
    }

    pub async fn ensure_authenticated(&mut self) -> Result<()> {
        // Already have a non-expired token in memory
        if self.access_token.is_some() {
            if !self.token_expires_at.map(|e| Utc::now() >= e).unwrap_or(false) {
                return Ok(());
            }
        }
        // Try DB
        if let Some((access, refresh, expires)) = self.db.get_token("google").await? {
            if !expires.map(|e| Utc::now() >= e).unwrap_or(true) {
                self.access_token     = Some(access);
                self.token_expires_at = expires;
                return Ok(());
            }
            if let Some(rt) = refresh {
                return self.refresh_token(&rt).await;
            }
        }
        Err(anyhow!("Not authenticated"))
    }

    async fn refresh_token(&mut self, refresh_token: &str) -> Result<()> {
        let rt = refresh_token.to_owned();
        let mut p = HashMap::new();
        p.insert("refresh_token", rt.as_str());
        p.insert("client_id",     CLIENT_ID);
        p.insert("client_secret", CLIENT_SECRET);
        p.insert("grant_type",    "refresh_token");

        let resp: TokenResponse = self.http.post(TOKEN_URL).form(&p)
            .send().await?.error_for_status()?.json().await?;
        self.store_tokens(resp).await
    }

    fn bearer(&self) -> String {
        format!("Bearer {}", self.access_token.as_deref().unwrap_or(""))
    }

    // ── Calendar API ──────────────────────────────────────────────────────────

    pub async fn pull_events(&mut self, calendar_id: &str) -> Result<Vec<GCalEvent>> {
        self.ensure_authenticated().await?;
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            pct(calendar_id)
        );
        let body: Value = self.http.get(&url)
            .header("Authorization", self.bearer())
            .query(&[
                ("singleEvents", "true"),
                ("orderBy",      "startTime"),
                ("maxResults",   "2500"),
            ])
            .send().await?.error_for_status()?.json().await?;

        Ok(body["items"].as_array().unwrap_or(&vec![]).iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect())
    }

    pub async fn push_event(&mut self, cal_id: &str, ev: &Event) -> Result<(String, String)> {
        self.ensure_authenticated().await?;
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            pct(cal_id)
        );
        let body: Value = self.http.post(&url)
            .header("Authorization", self.bearer())
            .json(&event_to_gcal(ev))
            .send().await?.error_for_status()?.json().await?;
        Ok((
            body["id"].as_str().unwrap_or("").to_owned(),
            body["etag"].as_str().unwrap_or("").to_owned(),
        ))
    }

    pub async fn update_event(
        &mut self, cal_id: &str, remote_id: &str, ev: &Event,
    ) -> Result<String> {
        self.ensure_authenticated().await?;
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            pct(cal_id), pct(remote_id)
        );
        let body: Value = self.http.put(&url)
            .header("Authorization", self.bearer())
            .json(&event_to_gcal(ev))
            .send().await?.error_for_status()?.json().await?;
        Ok(body["etag"].as_str().unwrap_or("").to_owned())
    }

    pub async fn delete_event(&mut self, cal_id: &str, remote_id: &str) -> Result<()> {
        self.ensure_authenticated().await?;
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            pct(cal_id), pct(remote_id)
        );
        self.http.delete(&url)
            .header("Authorization", self.bearer())
            .send().await?.error_for_status()?;
        Ok(())
    }

    // ── Tasks API ─────────────────────────────────────────────────────────────

    pub async fn pull_tasks(&mut self, task_list_id: &str) -> Result<Vec<GTask>> {
        self.ensure_authenticated().await?;
        let url = format!(
            "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks",
            pct(task_list_id)
        );
        let body: Value = self.http.get(&url)
            .header("Authorization", self.bearer())
            .query(&[
                ("showCompleted", "true"),
                ("showHidden",    "true"),
                ("showDeleted",   "true"),
                ("maxResults",    "100"),
            ])
            .send().await?.error_for_status()?.json().await?;

        Ok(body["items"].as_array().unwrap_or(&vec![]).iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect())
    }

    pub async fn push_task(
        &mut self, task_list_id: &str, task: &Task,
    ) -> Result<(String, String)> {
        self.ensure_authenticated().await?;
        let url = format!(
            "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks",
            pct(task_list_id)
        );
        let body: Value = self.http.post(&url)
            .header("Authorization", self.bearer())
            .json(&task_to_gtask(task))
            .send().await?.error_for_status()?.json().await?;
        Ok((
            body["id"].as_str().unwrap_or("").to_owned(),
            body["etag"].as_str().unwrap_or("").to_owned(),
        ))
    }

    pub async fn update_task(
        &mut self, task_list_id: &str, remote_id: &str, task: &Task,
    ) -> Result<String> {
        self.ensure_authenticated().await?;
        let url = format!(
            "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks/{}",
            pct(task_list_id), pct(remote_id)
        );
        let body: Value = self.http.put(&url)
            .header("Authorization", self.bearer())
            .json(&task_to_gtask(task))
            .send().await?.error_for_status()?.json().await?;
        Ok(body["etag"].as_str().unwrap_or("").to_owned())
    }

    pub async fn delete_task(&mut self, task_list_id: &str, remote_id: &str) -> Result<()> {
        self.ensure_authenticated().await?;
        let url = format!(
            "https://tasks.googleapis.com/tasks/v1/lists/{}/tasks/{}",
            pct(task_list_id), pct(remote_id)
        );
        self.http.delete(&url)
            .header("Authorization", self.bearer())
            .send().await?.error_for_status()?;
        Ok(())
    }
}

// ─── Calendar converters ──────────────────────────────────────────────────────

fn event_to_gcal(ev: &Event) -> Value {
    serde_json::json!({
        "summary":     ev.title,
        "description": ev.description,
        "start": if ev.all_day {
            serde_json::json!({ "date": ev.start.format("%Y-%m-%d").to_string() })
        } else {
            serde_json::json!({ "dateTime": ev.start.to_rfc3339(), "timeZone": "UTC" })
        },
        "end": if ev.all_day {
            serde_json::json!({ "date": ev.end.format("%Y-%m-%d").to_string() })
        } else {
            serde_json::json!({ "dateTime": ev.end.to_rfc3339(), "timeZone": "UTC" })
        },
    })
}

pub fn gcal_to_local(g: &GCalEvent, calendar_id: &str) -> Option<Event> {
    let title   = g.summary.clone().unwrap_or_else(|| "(no title)".into());
    let start   = parse_gcal_dt(g.start.as_ref()?)?;
    let end     = parse_gcal_dt(g.end.as_ref()?)?;
    let all_day = g.start.as_ref()?.date.is_some();
    let deleted = g.status.as_deref() == Some("cancelled");
    let now     = Utc::now();
    Some(Event {
        id: uuid::Uuid::new_v4().to_string(), title,
        description: g.description.clone(), start, end, all_day,
        calendar_id: Some(calendar_id.to_owned()),
        sync_id: g.id.clone(), etag: g.etag.clone(),
        dirty: false, deleted, created_at: now, updated_at: now,
    })
}

fn parse_gcal_dt(dt: &GCalDateTime) -> Option<DateTime<Utc>> {
    if let Some(s) = &dt.date_time {
        DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))
    } else if let Some(s) = &dt.date {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|d| d.and_utc())
    } else {
        None
    }
}

// ─── Tasks converters ─────────────────────────────────────────────────────────

fn task_to_gtask(t: &Task) -> Value {
    serde_json::json!({
        "title":  t.title,
        "notes":  t.notes,
        "status": if t.completed { "completed" } else { "needsAction" },
        "due":    t.due.map(|d| d.to_rfc3339()),
    })
}

pub fn gtask_to_local(g: &GTask, task_list_id: &str) -> Option<Task> {
    let deleted   = g.deleted.unwrap_or(false);
    let title     = g.title.clone().unwrap_or_else(|| "(no title)".into());
    let completed = g.status.as_deref() == Some("completed");
    let due       = g.due.as_ref().and_then(|s| {
        DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))
    });
    let now = Utc::now();
    Some(Task {
        id: uuid::Uuid::new_v4().to_string(), title,
        notes: g.notes.clone(), due, completed, priority: 0,
        task_list_id: Some(task_list_id.to_owned()),
        sync_id: g.id.clone(), dirty: false, deleted,
        created_at: now, updated_at: now,
    })
}

// ─── Utilities ────────────────────────────────────────────────────────────────

/// Minimal percent-encoding for URL path components.
fn pct(s: &str) -> String {
    s.chars().flat_map(|c| {
        if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            vec![c]
        } else {
            format!("%{:02X}", c as u32).chars().collect()
        }
    }).collect()
}
