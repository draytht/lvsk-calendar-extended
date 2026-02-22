use chrono::{Datelike, NaiveDate};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{block::Title, Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Panel};
use crate::calendar::days_in_month;

// ─── UI enums / state ─────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone, PartialEq)]
pub enum InputMode { #[default] Normal, Insert }

#[derive(Debug, Default, Clone, PartialEq)]
pub enum EventFormStep { #[default] Title, StartTime, EndTime }

#[derive(Debug, Default, Clone, PartialEq)]
pub enum TimeField { #[default] Hour, Minute }

#[derive(Debug, Clone)]
pub struct UiState {
    pub input_mode:      InputMode,
    pub new_event_title: String,
    pub new_task_title:  String,
    pub event_form_step: EventFormStep,
    pub event_start_h:   u32,
    pub event_start_m:   u32,
    pub event_end_h:     u32,
    pub event_end_m:     u32,
    pub time_field:      TimeField,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            input_mode:      InputMode::Normal,
            new_event_title: String::new(),
            new_task_title:  String::new(),
            event_form_step: EventFormStep::Title,
            event_start_h:   9,
            event_start_m:   0,
            event_end_h:     10,
            event_end_m:     0,
            time_field:      TimeField::Hour,
        }
    }
}

// ─── Root draw ────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Background fill
    f.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg())),
        area,
    );

    // root: [ header(3) | main | statusbar(1) ]
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // main: [ calendar(40) | right_panel ]
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(40), Constraint::Min(0)])
        .split(root[1]);

    // right_panel: [ events(58%) | tasks(42%) ]
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(cols[1]);

    draw_header(f, app, root[0]);
    draw_calendar(f, app, cols[0]);
    draw_events(f, app, rows[0]);
    draw_tasks(f, app, rows[1]);
    draw_statusbar(f, app, root[2]);

    // Overlays
    match app.active_panel {
        Panel::EventDetail => draw_event_form(f, area, app),
        Panel::TaskDetail  => draw_task_popup(f, area, app),
        Panel::Help        => draw_help(f, area, app),
        _ => {}
    }
}

// ─── Header bar ───────────────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let t  = &app.theme;
    let bt = app.theme.border_type();

    let is_hacker    = app.theme.name == "hacker";
    let is_cyberpunk = app.theme.name == "cyberpunk";

    let app_label = if is_hacker {
        " > LIFEMANAGER "
    } else if is_cyberpunk {
        " ⚡ LIFEMANAGER "
    } else {
        " ⬡ LifeManager "
    };

    let date_str   = app.selected_date.format("%A, %B %-d  %Y").to_string();
    let theme_str  = format!("  {}  ", app.theme.name);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(bt)
        .border_style(Style::default().fg(t.border_active()))
        .style(Style::default().bg(t.bg()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let sep = Style::default().fg(t.border());

    let line = Line::from(vec![
        Span::styled(app_label, Style::default().fg(t.accent()).add_modifier(Modifier::BOLD)),
        Span::styled("│", sep),
        Span::styled(format!("  {date_str}  "), Style::default().fg(t.fg())),
        Span::styled("│", sep),
        Span::styled(theme_str, Style::default().fg(t.fg_dim())),
        Span::styled("│", sep),
        Span::styled("  T: change theme  ", Style::default().fg(t.muted())),
    ]);

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(t.bg())),
        inner,
    );
}

// ─── Calendar ─────────────────────────────────────────────────────────────────

fn draw_calendar(f: &mut Frame, app: &App, area: Rect) {
    let t       = &app.theme;
    let focused = app.active_panel == Panel::Calendar;
    let bt      = app.theme.border_type();
    let bs      = Style::default().fg(if focused { t.border_active() } else { t.border() });
    let month_s = month_name(app.view_month);

    let title = Line::from(vec![
        Span::styled(
            format!(" {month_s} {} ", app.view_year),
            Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
        ),
    ]);

    // Legend in title-right area
    let legend = Line::from(vec![
        Span::styled(" ★ holiday ", Style::default().fg(t.holiday())),
        Span::styled("· event ", Style::default().fg(t.event_color())),
    ]);

    let block = Block::default()
        .title(Title::from(title).alignment(Alignment::Left))
        .title(Title::from(legend).alignment(Alignment::Right))
        .borders(Borders::ALL)
        .border_type(bt)
        .border_style(bs)
        .style(Style::default().bg(t.bg()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Day-of-week header
    let hdrs: Vec<Span> = ["Mo","Tu","We","Th","Fr","Sa","Su"].iter().enumerate().map(|(i, d)| {
        let st = if i >= 5 {
            Style::default().fg(t.weekend_color()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.fg_dim()).add_modifier(Modifier::BOLD)
        };
        Span::styled(format!(" {d} "), st)
    }).collect();
    lines.push(Line::from(hdrs));
    lines.push(Line::from(Span::styled(
        "─".repeat(inner.width as usize),
        Style::default().fg(t.border()),
    )));

    let first  = NaiveDate::from_ymd_opt(app.view_year, app.view_month, 1).unwrap();
    let offset = first.weekday().num_days_from_monday() as i32;
    let total  = days_in_month(app.view_year, app.view_month) as i32;
    let today  = chrono::Local::now().date_naive();

    for row in 0..6i32 {
        let row_start = row * 7 - offset + 1;
        if row_start > total { break; }

        let spans: Vec<Span> = (0..7i32).map(|col| {
            let d = row * 7 + col - offset + 1;
            if d < 1 || d > total {
                return Span::raw("    ");
            }

            let date    = NaiveDate::from_ymd_opt(app.view_year, app.view_month, d as u32).unwrap();
            let is_hol  = app.month_holidays.iter().any(|(hd, _)| *hd == d as u32);
            let has_ev  = app.month_event_days.contains(&(d as u32));
            let indicator = if is_hol { "★" } else if has_ev { "·" } else { " " };
            let label   = format!(" {:2}{}", d, indicator);

            let style = if date == app.selected_date {
                let (bg, fg) = t.selected_highlight();
                Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD)
            } else if date == today {
                let (bg, fg) = t.today_highlight();
                Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD)
            } else if is_hol {
                Style::default().fg(t.holiday()).add_modifier(Modifier::BOLD)
            } else if has_ev {
                Style::default().fg(t.event_color())
            } else if col >= 5 {
                Style::default().fg(t.weekend_color())
            } else {
                Style::default().fg(t.fg())
            };
            Span::styled(label, style)
        }).collect();

        lines.push(Line::from(spans));
    }

    // Upcoming holidays in this month (compact list at bottom)
    let today_day = if today.year() == app.view_year && today.month() == app.view_month {
        today.day()
    } else {
        0
    };
    let upcoming: Vec<_> = app.month_holidays.iter()
        .filter(|(d, _)| *d >= today_day)
        .take(3)
        .collect();

    if !upcoming.is_empty() {
        lines.push(Line::from(Span::styled(
            "─".repeat(inner.width as usize),
            Style::default().fg(t.border()),
        )));
        for (day, hol) in &upcoming {
            let mn = &month_name(app.view_month)[..3];
            lines.push(Line::from(vec![
                Span::styled(format!(" {mn} {:2} ", day), Style::default().fg(t.fg_dim())),
                Span::styled(hol.name, Style::default().fg(t.holiday())),
                Span::styled(format!(" [{}]", hol.country), Style::default().fg(t.muted())),
            ]));
        }
    }

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(t.bg())).alignment(Alignment::Left),
        inner,
    );
}

// ─── Events panel ─────────────────────────────────────────────────────────────

fn draw_events(f: &mut Frame, app: &App, area: Rect) {
    let t       = &app.theme;
    let focused = app.active_panel == Panel::EventList;
    let bt      = app.theme.border_type();
    let bs      = Style::default().fg(if focused { t.border_active() } else { t.border() });
    let date_s  = app.selected_date.format("%a %-d %b").to_string();

    let title = Line::from(vec![
        Span::styled(
            format!(" ● Events — {date_s} "),
            Style::default().fg(t.accent()),
        ),
    ]);

    let block = Block::default()
        .title(Title::from(title))
        .borders(Borders::ALL)
        .border_type(bt)
        .border_style(bs)
        .style(Style::default().bg(t.bg()));

    let hol_style  = Style::default().fg(t.holiday()).add_modifier(Modifier::BOLD);
    let hol_dim    = Style::default().fg(t.holiday());
    let mut items: Vec<ListItem> = Vec::new();

    // Show holidays for this day first
    for h in &app.selected_holidays {
        items.push(ListItem::new(Line::from(vec![
            Span::styled(" ★ ", hol_style),
            Span::styled(h.name, hol_style),
            Span::styled(format!("  [{}]", h.country), Style::default().fg(t.muted())),
        ])));
    }

    // Separator between holidays and events
    if !app.selected_holidays.is_empty() && !app.events.is_empty() {
        items.push(ListItem::new(Line::from(
            Span::styled("   ─────────────────", Style::default().fg(t.border()))
        )));
    }

    if app.events.is_empty() && app.selected_holidays.is_empty() {
        f.render_widget(
            Paragraph::new("  No events")
                .block(block)
                .style(Style::default().fg(t.fg_dim())),
            area,
        );
        return;
    }

    for (i, ev) in app.events.iter().enumerate() {
        let time = if ev.all_day {
            "all-day".to_owned()
        } else {
            ev.start.with_timezone(&chrono::Local).format("%H:%M").to_string()
        };
        let sel      = i == app.event_cursor && focused;
        let (bg, fg) = t.selected_highlight();
        let ts       = if sel {
            Style::default().bg(bg).fg(fg)
        } else {
            Style::default().fg(t.fg())
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(" ● ", Style::default().fg(t.event_color())),
            Span::styled(format!("{time} "), Style::default().fg(t.fg_dim())),
            Span::styled(ev.title.clone(), ts),
        ])));
    }

    let mut state = ListState::default();
    // offset cursor past holiday items when selecting
    let ev_offset = app.selected_holidays.len()
        + if !app.selected_holidays.is_empty() && !app.events.is_empty() { 1 } else { 0 };
    state.select(if focused { Some(app.event_cursor + ev_offset) } else { None });
    f.render_stateful_widget(List::new(items).block(block), area, &mut state);
}

// ─── Tasks panel ──────────────────────────────────────────────────────────────

fn draw_tasks(f: &mut Frame, app: &App, area: Rect) {
    let t       = &app.theme;
    let focused = app.active_panel == Panel::TaskList;
    let bt      = app.theme.border_type();
    let bs      = Style::default().fg(if focused { t.border_active() } else { t.border() });

    let title = Line::from(Span::styled(" ○ Tasks ", Style::default().fg(t.accent())));

    let block = Block::default()
        .title(Title::from(title))
        .borders(Borders::ALL)
        .border_type(bt)
        .border_style(bs)
        .style(Style::default().bg(t.bg()));

    if app.tasks.is_empty() {
        f.render_widget(
            Paragraph::new("  No tasks")
                .block(block)
                .style(Style::default().fg(t.fg_dim())),
            area,
        );
        return;
    }

    let now = chrono::Utc::now();
    let items: Vec<ListItem> = app.tasks.iter().enumerate().map(|(i, task)| {
        let check = if task.completed { " ✔ " } else { " ○ " };
        let cs    = if task.completed {
            Style::default().fg(t.event_color())
        } else {
            Style::default().fg(t.fg_dim())
        };
        let sel      = i == app.task_cursor && focused;
        let (bg, fg) = t.selected_highlight();

        let is_overdue = !task.completed && task.due.map(|d| d < now).unwrap_or(false);

        let ts = if task.completed {
            Style::default().fg(t.fg_dim()).add_modifier(Modifier::CROSSED_OUT)
        } else if is_overdue {
            Style::default().fg(t.error()).add_modifier(Modifier::BOLD)
        } else if sel {
            Style::default().bg(bg).fg(fg)
        } else {
            Style::default().fg(t.fg())
        };

        // Optional due date suffix
        let due_suffix = task.due.map(|d| {
            let local = d.with_timezone(&chrono::Local);
            format!("  due {}", local.format("%-d %b"))
        }).unwrap_or_default();

        ListItem::new(Line::from(vec![
            Span::styled(check, cs),
            Span::styled(task.title.clone(), ts),
            Span::styled(due_suffix, Style::default().fg(
                if is_overdue { t.error() } else { t.muted() }
            )),
        ]))
    }).collect();

    let mut state = ListState::default();
    state.select(if focused { Some(app.task_cursor) } else { None });
    f.render_stateful_widget(List::new(items).block(block), area, &mut state);
}

// ─── Status bar ───────────────────────────────────────────────────────────────

fn draw_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let (mode_str, mode_style) = match app.ui.input_mode {
        InputMode::Normal => (
            " NORMAL ",
            Style::default().bg(t.accent()).fg(t.bg()).add_modifier(Modifier::BOLD),
        ),
        InputMode::Insert => (
            " INSERT ",
            Style::default().bg(t.event_color()).fg(t.bg()).add_modifier(Modifier::BOLD),
        ),
    };
    let bar = Paragraph::new(Line::from(vec![
        Span::styled(mode_str, mode_style),
        Span::styled(
            "  hjkl:nav  n:event  N:task  T:theme  Space:done  d:del  Tab  [:prev  ]:next  t:today  ?:help  ^s:sync",
            Style::default().fg(t.fg_dim()),
        ),
        Span::styled(
            format!("  {}", app.sync_status),
            Style::default().fg(t.muted()).add_modifier(Modifier::ITALIC),
        ),
    ])).style(Style::default().bg(t.bg2()));
    f.render_widget(bar, area);
}

// ─── Shadow helper ────────────────────────────────────────────────────────────

fn draw_shadow(f: &mut Frame, rect: Rect, color: ratatui::style::Color) {
    let area = f.area();
    // Right shadow
    if rect.right() < area.right() {
        let sv = Rect {
            x: rect.right(),
            y: rect.y.saturating_add(1),
            width: 1,
            height: rect.height.min(area.height.saturating_sub(rect.y.saturating_add(1))),
        };
        if sv.height > 0 {
            f.render_widget(Block::default().style(Style::default().bg(color)), sv);
        }
    }
    // Bottom shadow
    if rect.bottom() < area.bottom() {
        let sh = Rect {
            x: rect.x.saturating_add(1),
            y: rect.bottom(),
            width: rect.width.min(area.width.saturating_sub(rect.x.saturating_add(1))),
            height: 1,
        };
        if sh.width > 0 {
            f.render_widget(Block::default().style(Style::default().bg(color)), sh);
        }
    }
}

// ─── Event creation form (multi-step) ────────────────────────────────────────

fn draw_event_form(f: &mut Frame, area: Rect, app: &App) {
    let t    = &app.theme;
    let bt   = app.theme.border_type();
    let rect = centered(62, 52, area);
    draw_shadow(f, rect, t.bg2());
    f.render_widget(Clear, rect);

    let block = Block::default()
        .title(Title::from(Line::from(Span::styled(
            " New Event ",
            Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
        ))))
        .borders(Borders::ALL)
        .border_type(bt)
        .border_style(Style::default().fg(t.border_active()))
        .style(Style::default().bg(t.popup_bg()));

    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let step       = &app.ui.event_form_step;
    let acc        = Style::default().fg(t.accent()).add_modifier(Modifier::BOLD);
    let dim        = Style::default().fg(t.fg_dim());
    let fg         = Style::default().fg(t.fg());
    let (sel_bg, sel_fg) = t.selected_highlight();
    let sel        = Style::default().bg(sel_bg).fg(sel_fg).add_modifier(Modifier::BOLD);
    let hour_focus = app.ui.time_field == TimeField::Hour;

    let title_active = *step == EventFormStep::Title;
    let start_active = *step == EventFormStep::StartTime;
    let end_active   = *step == EventFormStep::EndTime;

    let step_label = match step {
        EventFormStep::Title     => "Step 1 / 3  —  Event title",
        EventFormStep::StartTime => "Step 2 / 3  —  Start time",
        EventFormStep::EndTime   => "Step 3 / 3  —  End time",
    };

    let title_val  = format!("{}{}", app.ui.new_event_title, if title_active { "█" } else { "" });

    // Start time spans
    let start_line: Line = if start_active {
        Line::from(vec![
            Span::styled("▶ Start   ", acc),
            Span::styled(format!("{:02}", app.ui.event_start_h), if hour_focus { sel } else { fg }),
            Span::styled("  :  ", dim),
            Span::styled(format!("{:02}", app.ui.event_start_m), if !hour_focus { sel } else { fg }),
        ])
    } else {
        Line::from(vec![
            Span::styled("  Start   ", dim),
            Span::styled(format!("{:02} : {:02}", app.ui.event_start_h, app.ui.event_start_m), dim),
        ])
    };

    // End time spans
    let end_line: Line = if end_active {
        Line::from(vec![
            Span::styled("▶ End     ", acc),
            Span::styled(format!("{:02}", app.ui.event_end_h), if hour_focus { sel } else { fg }),
            Span::styled("  :  ", dim),
            Span::styled(format!("{:02}", app.ui.event_end_m), if !hour_focus { sel } else { fg }),
        ])
    } else {
        Line::from(vec![
            Span::styled("  End     ", dim),
            Span::styled(format!("{:02} : {:02}", app.ui.event_end_h, app.ui.event_end_m), dim),
        ])
    };

    let hint: Line = match step {
        EventFormStep::Title =>
            Line::from(Span::styled("  Enter: next   Esc: cancel", dim)),
        EventFormStep::StartTime | EventFormStep::EndTime =>
            Line::from(Span::styled("  ↑↓ adjust   ←→ hour/min   Enter: next", dim)),
    };
    let hint_enter: Line = if *step == EventFormStep::EndTime {
        Line::from(Span::styled("  Enter: save event", Style::default().fg(t.accent())))
    } else {
        Line::from("")
    };

    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {step_label}"), Style::default().fg(t.muted()))),
        Line::from(""),
        Line::from(vec![
            Span::styled(if title_active { "▶ Title   " } else { "  Title   " }, if title_active { acc } else { dim }),
            Span::styled(title_val, if title_active { fg } else { dim }),
        ]),
        Line::from(""),
        start_line,
        Line::from(""),
        end_line,
        Line::from(""),
        Line::from(Span::styled("─".repeat(inner.width.saturating_sub(2) as usize), dim)),
        Line::from(""),
        hint,
        hint_enter,
    ];

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(t.popup_bg())),
        inner,
    );
}

// ─── Task input popup ─────────────────────────────────────────────────────────

fn draw_task_popup(f: &mut Frame, area: Rect, app: &App) {
    let t    = &app.theme;
    let bt   = app.theme.border_type();
    let rect = centered(58, 20, area);
    draw_shadow(f, rect, t.bg2());
    f.render_widget(Clear, rect);

    let block = Block::default()
        .title(Title::from(Line::from(Span::styled(
            " New Task ",
            Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
        ))))
        .borders(Borders::ALL)
        .border_type(bt)
        .border_style(Style::default().fg(t.border_active()))
        .style(Style::default().bg(t.popup_bg()));

    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let dim = Style::default().fg(t.fg_dim());
    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  ○ ", dim),
            Span::styled(
                format!("{}█", app.ui.new_task_title),
                Style::default().fg(t.fg()),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Enter: save   Esc: cancel", dim)),
    ];
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(t.popup_bg())),
        inner,
    );
}

// ─── Help overlay ─────────────────────────────────────────────────────────────

fn draw_help(f: &mut Frame, area: Rect, app: &App) {
    let t    = &app.theme;
    let bt   = app.theme.border_type();
    let rect = centered(70, 88, area);
    draw_shadow(f, rect, t.bg2());
    f.render_widget(Clear, rect);

    let block = Block::default()
        .title(Title::from(Line::from(Span::styled(
            " Keyboard Shortcuts ",
            Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
        ))))
        .borders(Borders::ALL)
        .border_type(bt)
        .border_style(Style::default().fg(t.border_active()))
        .style(Style::default().bg(t.popup_bg()));

    let acc = Style::default().fg(t.accent()).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(t.fg_dim());
    let hol = Style::default().fg(t.holiday());

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", acc)),
        Line::from(Span::styled("  h / j / k / l   ←↓↑→    Move by day / week", dim)),
        Line::from(Span::styled("  [ / ]            Prev / Next month", dim)),
        Line::from(Span::styled("  t                Jump to today", dim)),
        Line::from(Span::styled("  Tab              Cycle panels", dim)),
        Line::from(""),
        Line::from(Span::styled("  Events", acc)),
        Line::from(Span::styled("  n                New event  (3-step form)", dim)),
        Line::from(Span::styled("    Enter            Next step", dim)),
        Line::from(Span::styled("    ↑ / ↓            Adjust hour or minute (15 min)", dim)),
        Line::from(Span::styled("    ← / →            Switch hour / minute field", dim)),
        Line::from(Span::styled("  d / Del          Delete selected event", dim)),
        Line::from(Span::styled("  Enter            Focus event list", dim)),
        Line::from(""),
        Line::from(Span::styled("  Tasks", acc)),
        Line::from(Span::styled("  N                New task", dim)),
        Line::from(Span::styled("  Space            Toggle complete / incomplete", dim)),
        Line::from(""),
        Line::from(Span::styled("  Themes  (8 built-in)", acc)),
        Line::from(Span::styled("  T  (Shift+T)     Cycle: Mocha → Nord → Gruvbox → Tokyo Night", dim)),
        Line::from(Span::styled("                         → Dracula → Cyberpunk → Hacker → Vietnam", dim)),
        Line::from(""),
        Line::from(Span::styled("  Sync  (Google Calendar + Tasks)", acc)),
        Line::from(Span::styled("  Ctrl+s           Force sync now", dim)),
        Line::from(Span::styled("  Auto-syncs every 5 min when configured", dim)),
        Line::from(""),
        Line::from(Span::styled("  Holidays  ★", hol)),
        Line::from(Span::styled("  US federal + cultural holidays", hol)),
        Line::from(Span::styled("  Vietnam public + lunar holidays", hol)),
        Line::from(""),
        Line::from(Span::styled("  General", acc)),
        Line::from(Span::styled("  ?                Toggle this help", dim)),
        Line::from(Span::styled("  Esc              Cancel / back", dim)),
        Line::from(Span::styled("  q                Quit", dim)),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .style(Style::default().fg(t.fg()))
            .wrap(Wrap { trim: false }),
        rect,
    );
}

// ─── Utilities ────────────────────────────────────────────────────────────────

fn centered(pct_x: u16, pct_y: u16, r: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(vert[1])[1]
}

fn month_name(m: u32) -> &'static str {
    match m {
        1=>"January", 2=>"February", 3=>"March",    4=>"April",
        5=>"May",     6=>"June",     7=>"July",      8=>"August",
        9=>"September",10=>"October",11=>"November",12=>"December",
        _=>"???",
    }
}
