use anyhow::Result;
use chrono::{Datelike, Duration, Local, NaiveDate};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashSet;
use std::io;

use crate::{
    db::{Database, Event as DbEvent, Task},
    holidays::{self, Holiday},
    sync::worker::{SyncEvent, SyncWorker},
    theme::ThemeConfig,
    ui::{draw, EventFormStep, InputMode, TimeField, UiState},
};

// ─── Panel focus model ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Calendar,
    EventList,
    TaskList,
    EventDetail,
    TaskDetail,
    Help,
}

// ─── App state ────────────────────────────────────────────────────────────────

pub struct App {
    pub db:               Database,
    pub theme:            ThemeConfig,
    pub theme_idx:        usize,
    pub sync:             Option<SyncWorker>,
    pub selected_date:    NaiveDate,
    pub view_month:       u32,
    pub view_year:        i32,
    pub active_panel:     Panel,
    pub events:           Vec<DbEvent>,
    pub tasks:            Vec<Task>,
    pub event_cursor:     usize,
    pub task_cursor:      usize,
    pub ui:               UiState,
    pub sync_status:      String,
    pub running:          bool,
    // Month-level data for calendar dots
    pub month_event_days: HashSet<u32>,
    pub month_holidays:   Vec<(u32, Holiday)>,
    // Selected-day holidays
    pub selected_holidays: Vec<Holiday>,
}

impl App {
    pub async fn new(db: Database, theme: ThemeConfig) -> Result<Self> {
        let today  = Local::now().date_naive();
        let events = db.events_in_range(
            today.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            today.and_hms_opt(23, 59, 59).unwrap().and_utc(),
        ).await.unwrap_or_default();
        let tasks = db.all_tasks().await.unwrap_or_default();

        let all     = ThemeConfig::all_themes();
        let idx     = all.iter().position(|t| t.name == theme.name).unwrap_or(0);
        let sel_hol = holidays::holidays_on(today);
        let mon_hol = holidays::holidays_in_month(today.year(), today.month());

        Ok(Self {
            theme_idx: idx,
            theme, db, sync: None,
            selected_date: today,
            view_month:    today.month(),
            view_year:     today.year(),
            active_panel:  Panel::Calendar,
            events, tasks,
            event_cursor: 0, task_cursor: 0,
            ui: UiState::default(),
            sync_status: String::new(),
            running: true,
            month_event_days:  HashSet::new(),
            month_holidays:    mon_hol,
            selected_holidays: sel_hol,
        })
    }

    pub fn attach_sync_worker(&mut self, w: SyncWorker) { self.sync = Some(w); }

    // ── TUI loop ──────────────────────────────────────────────────────────────

    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend  = CrosstermBackend::new(stdout);
        let mut term = Terminal::new(backend)?;

        let result = self.event_loop(&mut term).await;

        disable_raw_mode()?;
        execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        term.show_cursor()?;
        result
    }

    async fn event_loop(
        &mut self,
        term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        // Load initial month event dots
        self.refresh_month().await;

        let tick = std::time::Duration::from_millis(50);
        while self.running {
            term.draw(|f| draw(f, self))?;

            let pending: Vec<SyncEvent> = if let Some(ref w) = self.sync {
                if let Ok(mut rx) = w.event_rx.try_lock() {
                    let mut buf = Vec::new();
                    while let Ok(ev) = rx.try_recv() { buf.push(ev); }
                    buf
                } else { vec![] }
            } else { vec![] };
            for ev in pending { self.on_sync_event(ev); }

            if event::poll(tick)? {
                if let Event::Key(key) = event::read()? {
                    self.on_key(key).await?;
                }
            }
        }

        if let Some(ref w) = self.sync { w.shutdown().await; }
        Ok(())
    }

    fn on_sync_event(&mut self, ev: SyncEvent) {
        self.sync_status = match ev {
            SyncEvent::SyncStarted                         => "⟳ Syncing…".into(),
            SyncEvent::SyncComplete { pulled, pushed } =>
                format!("✓ +{pulled} pulled  {pushed} pushed"),
            SyncEvent::SyncError(msg)                      => format!("✗ {msg}"),
            SyncEvent::AuthRequired                        => "Auth required — run: lm auth google".into(),
        };
    }

    // ── Input ─────────────────────────────────────────────────────────────────

    async fn on_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), _) => { self.running = false; return Ok(()); }
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                if let Some(ref w) = self.sync { w.sync_now().await; }
                return Ok(());
            }
            (KeyCode::Char('?'), _) => { self.active_panel = Panel::Help; return Ok(()); }
            (KeyCode::Esc, _) => {
                self.active_panel       = Panel::Calendar;
                self.ui.input_mode      = InputMode::Normal;
                self.ui.event_form_step = EventFormStep::Title;
                return Ok(());
            }
            _ => {}
        }

        let panel = self.active_panel.clone();
        match panel {
            Panel::Calendar     => self.key_calendar(key).await?,
            Panel::EventList    => self.key_events(key).await?,
            Panel::TaskList     => self.key_tasks(key).await?,
            Panel::EventDetail
            | Panel::TaskDetail => self.key_form(key).await?,
            Panel::Help         => {}
        }
        Ok(())
    }

    async fn key_calendar(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Right | KeyCode::Char('l') => self.shift_day(1).await,
            KeyCode::Left  | KeyCode::Char('h') => self.shift_day(-1).await,
            KeyCode::Down  | KeyCode::Char('j') => self.shift_day(7).await,
            KeyCode::Up    | KeyCode::Char('k') => self.shift_day(-7).await,
            KeyCode::Char(']') => { self.next_month(); self.refresh_month().await; }
            KeyCode::Char('[') => { self.prev_month(); self.refresh_month().await; }
            KeyCode::Char('t') => {
                let t = Local::now().date_naive();
                self.selected_date = t;
                self.view_month    = t.month();
                self.view_year     = t.year();
                self.refresh().await;
            }
            // T (Shift+T) — cycle through themes
            KeyCode::Char('T') => {
                let themes = ThemeConfig::all_themes();
                self.theme_idx = (self.theme_idx + 1) % themes.len();
                self.theme     = themes[self.theme_idx].clone();
                let _ = self.theme.save();
            }
            KeyCode::Enter => self.active_panel = Panel::EventList,
            KeyCode::Tab   => self.active_panel = Panel::TaskList,
            KeyCode::Char('n') => {
                self.ui.new_event_title.clear();
                self.ui.event_form_step = EventFormStep::Title;
                self.ui.event_start_h   = 9;
                self.ui.event_start_m   = 0;
                self.ui.event_end_h     = 10;
                self.ui.event_end_m     = 0;
                self.ui.time_field      = TimeField::Hour;
                self.ui.input_mode      = InputMode::Insert;
                self.active_panel       = Panel::EventDetail;
            }
            KeyCode::Char('N') => {
                self.ui.new_task_title.clear();
                self.ui.input_mode = InputMode::Insert;
                self.active_panel  = Panel::TaskDetail;
            }
            _ => {}
        }
        Ok(())
    }

    async fn key_events(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if self.event_cursor + 1 < self.events.len() { self.event_cursor += 1; }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.event_cursor = self.event_cursor.saturating_sub(1);
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(ev) = self.events.get(self.event_cursor).cloned() {
                    let mut e = ev;
                    e.deleted = true;
                    e.dirty   = true;
                    self.db.upsert_event(&e).await?;
                    self.refresh().await;
                    if let Some(ref w) = self.sync { w.push_dirty().await; }
                }
            }
            KeyCode::Tab => self.active_panel = Panel::TaskList,
            _            => self.active_panel = Panel::Calendar,
        }
        Ok(())
    }

    async fn key_tasks(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if self.task_cursor + 1 < self.tasks.len() { self.task_cursor += 1; }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.task_cursor = self.task_cursor.saturating_sub(1);
            }
            KeyCode::Char(' ') => {
                if let Some(t) = self.tasks.get(self.task_cursor).cloned() {
                    let mut t    = t;
                    t.completed  = !t.completed;
                    t.dirty      = true;
                    t.updated_at = chrono::Utc::now();
                    self.db.upsert_task(&t).await?;
                    self.refresh().await;
                    if let Some(ref w) = self.sync { w.push_dirty().await; }
                }
            }
            KeyCode::Tab => self.active_panel = Panel::Calendar,
            _            => self.active_panel = Panel::Calendar,
        }
        Ok(())
    }

    // ── Multi-step event form ─────────────────────────────────────────────────

    async fn key_form(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        if self.ui.input_mode != InputMode::Insert {
            self.active_panel = Panel::Calendar;
            return Ok(());
        }

        match self.active_panel {
            Panel::TaskDetail => match key.code {
                KeyCode::Char(c)   => self.ui.new_task_title.push(c),
                KeyCode::Backspace => { self.ui.new_task_title.pop(); }
                KeyCode::Enter     => self.commit_form().await?,
                _ => {}
            },
            Panel::EventDetail => match self.ui.event_form_step {
                EventFormStep::Title => match key.code {
                    KeyCode::Char(c)   => self.ui.new_event_title.push(c),
                    KeyCode::Backspace => { self.ui.new_event_title.pop(); }
                    KeyCode::Enter => {
                        if !self.ui.new_event_title.trim().is_empty() {
                            self.ui.event_form_step = EventFormStep::StartTime;
                            self.ui.time_field      = TimeField::Hour;
                        }
                    }
                    _ => {}
                },
                EventFormStep::StartTime => match key.code {
                    KeyCode::Up    | KeyCode::Char('k') => self.adjust_start_time(1),
                    KeyCode::Down  | KeyCode::Char('j') => self.adjust_start_time(-1),
                    KeyCode::Left  | KeyCode::Char('h') => self.ui.time_field = TimeField::Hour,
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                        self.ui.time_field = TimeField::Minute;
                    }
                    KeyCode::Enter => {
                        self.ui.event_form_step = EventFormStep::EndTime;
                        self.ui.time_field      = TimeField::Hour;
                    }
                    _ => {}
                },
                EventFormStep::EndTime => match key.code {
                    KeyCode::Up    | KeyCode::Char('k') => self.adjust_end_time(1),
                    KeyCode::Down  | KeyCode::Char('j') => self.adjust_end_time(-1),
                    KeyCode::Left  | KeyCode::Char('h') => self.ui.time_field = TimeField::Hour,
                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                        self.ui.time_field = TimeField::Minute;
                    }
                    KeyCode::Enter => self.commit_form().await?,
                    _ => {}
                },
            },
            _ => {}
        }
        Ok(())
    }

    fn adjust_start_time(&mut self, delta: i32) {
        match self.ui.time_field {
            TimeField::Hour   => {
                self.ui.event_start_h =
                    ((self.ui.event_start_h as i32 + delta).rem_euclid(24)) as u32;
            }
            TimeField::Minute => {
                self.ui.event_start_m =
                    ((self.ui.event_start_m as i32 + delta * 15).rem_euclid(60)) as u32;
            }
        }
    }

    fn adjust_end_time(&mut self, delta: i32) {
        match self.ui.time_field {
            TimeField::Hour   => {
                self.ui.event_end_h =
                    ((self.ui.event_end_h as i32 + delta).rem_euclid(24)) as u32;
            }
            TimeField::Minute => {
                self.ui.event_end_m =
                    ((self.ui.event_end_m as i32 + delta * 15).rem_euclid(60)) as u32;
            }
        }
    }

    async fn commit_form(&mut self) -> Result<()> {
        match self.active_panel {
            Panel::EventDetail => {
                let title = self.ui.new_event_title.trim().to_owned();
                if !title.is_empty() {
                    let start = self.selected_date
                        .and_hms_opt(self.ui.event_start_h, self.ui.event_start_m, 0)
                        .unwrap().and_utc();
                    let end = self.selected_date
                        .and_hms_opt(self.ui.event_end_h, self.ui.event_end_m, 0)
                        .unwrap().and_utc();
                    self.db.upsert_event(&DbEvent::new(&title, start, end)).await?;
                    if let Some(ref w) = self.sync { w.push_dirty().await; }
                }
                self.ui.event_form_step = EventFormStep::Title;
            }
            Panel::TaskDetail => {
                let title = self.ui.new_task_title.trim().to_owned();
                if !title.is_empty() {
                    self.db.upsert_task(&Task::new(&title)).await?;
                    if let Some(ref w) = self.sync { w.push_dirty().await; }
                }
            }
            _ => {}
        }
        self.ui.input_mode = InputMode::Normal;
        self.active_panel  = Panel::Calendar;
        self.refresh().await;
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    async fn shift_day(&mut self, d: i64) {
        let date = self.selected_date + Duration::days(d);
        let prev_month = self.view_month;
        let prev_year  = self.view_year;
        self.selected_date = date;
        self.view_month    = date.month();
        self.view_year     = date.year();
        if self.view_month != prev_month || self.view_year != prev_year {
            self.refresh_month().await;
        }
        self.refresh().await;
    }

    fn next_month(&mut self) {
        if self.view_month == 12 { self.view_month = 1;  self.view_year += 1; }
        else                     { self.view_month += 1; }
    }

    fn prev_month(&mut self) {
        if self.view_month == 1 { self.view_month = 12; self.view_year -= 1; }
        else                    { self.view_month -= 1; }
    }

    async fn refresh(&mut self) {
        let s = self.selected_date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let e = self.selected_date.and_hms_opt(23, 59, 59).unwrap().and_utc();
        self.events            = self.db.events_in_range(s, e).await.unwrap_or_default();
        self.tasks             = self.db.all_tasks().await.unwrap_or_default();
        self.event_cursor      = 0;
        self.task_cursor       = 0;
        self.selected_holidays = holidays::holidays_on(self.selected_date);
    }

    /// Refresh which days in the current view-month have events/holidays.
    pub async fn refresh_month(&mut self) {
        let start = NaiveDate::from_ymd_opt(self.view_year, self.view_month, 1)
            .unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let end = if self.view_month == 12 {
            NaiveDate::from_ymd_opt(self.view_year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(self.view_year, self.view_month + 1, 1)
        }.unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc();

        let evs = self.db.events_in_range(start, end).await.unwrap_or_default();
        self.month_event_days = evs.iter()
            .map(|e| e.start.with_timezone(&chrono::Local).day())
            .collect();
        self.month_holidays = holidays::holidays_in_month(self.view_year, self.view_month);
    }
}
