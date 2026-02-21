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

/// Which step of the multi-step event creation form we're on.
#[derive(Debug, Default, Clone, PartialEq)]
pub enum EventFormStep {
    #[default]
    Title,
    StartTime,
    EndTime,
}

/// Which time field (hour or minute) is focused in the time picker.
#[derive(Debug, Default, Clone, PartialEq)]
pub enum TimeField { #[default] Hour, Minute }

#[derive(Debug, Clone)]
pub struct UiState {
    pub input_mode:      InputMode,
    pub new_event_title: String,
    pub new_task_title:  String,
    // Time-picker state (event form steps 2 & 3)
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

    // Fill background
    f.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg()).fg(app.theme.fg())),
        area,
    );

    // Layout: [ content | status_bar(1) ]
    let root = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)]).split(area);

    // Content: [ calendar(34) | right_panel ]
    let cols = Layout::default().direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(0)]).split(root[0]);

    // Right: [ events(50%) | tasks(50%) ]
    let rows = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[1]);

    draw_calendar(f, app, cols[0]);
    draw_events(f, app, rows[0]);
    draw_tasks(f, app, rows[1]);
    draw_statusbar(f, app, root[1]);

    // Overlays
    match app.active_panel {
        Panel::EventDetail => draw_event_form(f, area, app),
        Panel::TaskDetail  => draw_popup(f, "New Task", &app.ui.new_task_title, area, app),
        Panel::Help        => draw_help(f, area, app),
        _ => {}
    }
}

// ─── Calendar ─────────────────────────────────────────────────────────────────

fn draw_calendar(f: &mut Frame, app: &App, area: Rect) {
    let t       = &app.theme;
    let focused = app.active_panel == Panel::Calendar;
    let bs      = Style::default().fg(if focused { t.border_active() } else { t.border() });
    let month_s = month_name(app.view_month);
    let title   = Line::from(Span::styled(
        format!(" {month_s} {} ", app.view_year),
        Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
    ));

    let block = Block::default()
        .title(Title::from(title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(bs)
        .style(Style::default().bg(t.bg()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Build calendar lines
    let mut lines: Vec<Line> = vec![];

    // Header row: Mo Tu We Th Fr Sa Su
    let hdrs: Vec<Span> = ["Mo","Tu","We","Th","Fr","Sa","Su"].iter().enumerate().map(|(i, d)| {
        let style = if i >= 5 {
            Style::default().fg(t.weekend_color()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.fg_dim()).add_modifier(Modifier::BOLD)
        };
        Span::styled(format!(" {d} "), style)
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
            let date  = NaiveDate::from_ymd_opt(app.view_year, app.view_month, d as u32).unwrap();
            let label = format!(" {:2} ", d);

            let style = if date == app.selected_date {
                let (bg, fg) = t.selected_highlight();
                Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD)
            } else if date == today {
                let (bg, fg) = t.today_highlight();
                Style::default().bg(bg).fg(fg).add_modifier(Modifier::BOLD)
            } else if col >= 5 {
                Style::default().fg(t.weekend_color())
            } else {
                Style::default().fg(t.fg())
            };
            Span::styled(label, style)
        }).collect();

        lines.push(Line::from(spans));
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
    let bs      = Style::default().fg(if focused { t.border_active() } else { t.border() });
    let date_s  = app.selected_date.format("%A, %B %-d").to_string();
    let title   = Line::from(Span::styled(
        format!(" ● Events — {date_s} "),
        Style::default().fg(t.accent()),
    ));

    let block = Block::default()
        .title(Title::from(title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(bs)
        .style(Style::default().bg(t.bg()));

    if app.events.is_empty() {
        f.render_widget(
            Paragraph::new("  No events").block(block).style(Style::default().fg(t.fg_dim())),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = app.events.iter().enumerate().map(|(i, ev)| {
        let time   = if ev.all_day {
            "all-day".to_owned()
        } else {
            ev.start.format("%H:%M").to_string()
        };
        let sel    = i == app.event_cursor && focused;
        let (bg, fg) = t.selected_highlight();
        let ts     = if sel { Style::default().bg(bg).fg(fg) } else { Style::default().fg(t.fg()) };
        ListItem::new(Line::from(vec![
            Span::styled(" ● ", Style::default().fg(t.event_color())),
            Span::styled(format!("{time} "), Style::default().fg(t.fg_dim())),
            Span::styled(ev.title.clone(), ts),
        ]))
    }).collect();

    let mut state = ListState::default();
    state.select(if focused { Some(app.event_cursor) } else { None });
    f.render_stateful_widget(List::new(items).block(block).highlight_symbol("▶ "), area, &mut state);
}

// ─── Tasks panel ──────────────────────────────────────────────────────────────

fn draw_tasks(f: &mut Frame, app: &App, area: Rect) {
    let t       = &app.theme;
    let focused = app.active_panel == Panel::TaskList;
    let bs      = Style::default().fg(if focused { t.border_active() } else { t.border() });
    let title   = Line::from(Span::styled(" ○ Tasks ", Style::default().fg(t.accent())));

    let block = Block::default()
        .title(Title::from(title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(bs)
        .style(Style::default().bg(t.bg()));

    if app.tasks.is_empty() {
        f.render_widget(
            Paragraph::new("  No tasks").block(block).style(Style::default().fg(t.fg_dim())),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = app.tasks.iter().enumerate().map(|(i, task)| {
        let check  = if task.completed { " ✔ " } else { " ○ " };
        let cs     = if task.completed {
            Style::default().fg(t.event_color())
        } else {
            Style::default().fg(t.fg_dim())
        };
        let sel      = i == app.task_cursor && focused;
        let (bg, fg) = t.selected_highlight();
        let ts       = if task.completed {
            Style::default().fg(t.fg_dim()).add_modifier(Modifier::CROSSED_OUT)
        } else if sel {
            Style::default().bg(bg).fg(fg)
        } else {
            Style::default().fg(t.fg())
        };
        ListItem::new(Line::from(vec![
            Span::styled(check, cs),
            Span::styled(task.title.clone(), ts),
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
        InputMode::Normal => (" NORMAL ", Style::default().bg(t.accent()).fg(t.bg()).add_modifier(Modifier::BOLD)),
        InputMode::Insert => (" INSERT ", Style::default().bg(t.event_color()).fg(t.bg()).add_modifier(Modifier::BOLD)),
    };
    let bar = Paragraph::new(Line::from(vec![
        Span::styled(mode_str, mode_style),
        Span::styled(
            "  hjkl:nav  n:event  N:task  Space:done  d:del  Tab:panels  [:prev  ]:next  t:today  ?:help  ^s:sync  q:quit",
            Style::default().fg(t.fg_dim()),
        ),
        Span::styled(
            format!("  {}", app.sync_status),
            Style::default().fg(t.muted()).add_modifier(Modifier::ITALIC),
        ),
    ])).style(Style::default().bg(t.bg2()));
    f.render_widget(bar, area);
}

// ─── Event creation form (multi-step) ────────────────────────────────────────

fn draw_event_form(f: &mut Frame, area: Rect, app: &App) {
    let t    = &app.theme;
    let rect = centered(60, 50, area);
    f.render_widget(Clear, rect);

    let block = Block::default()
        .title(Title::from(Line::from(Span::styled(
            " New Event ",
            Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
        ))))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_active()))
        .style(Style::default().bg(t.popup_bg()));

    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let step = &app.ui.event_form_step;

    let acc = Style::default().fg(t.accent()).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(t.fg_dim());
    let fg  = Style::default().fg(t.fg());
    let (sel_bg, sel_fg) = t.selected_highlight();
    let sel = Style::default().bg(sel_bg).fg(sel_fg).add_modifier(Modifier::BOLD);

    let title_active = *step == EventFormStep::Title;
    let start_active = *step == EventFormStep::StartTime;
    let end_active   = *step == EventFormStep::EndTime;
    let hour_focus   = app.ui.time_field == TimeField::Hour;

    // ── Title row ────────────────────────────────────────────────────────────
    let title_prefix = if title_active { "▶ Title  " } else { "  Title  " };
    let title_val = format!(
        "{}{}",
        app.ui.new_event_title,
        if title_active { "█" } else { "" }
    );
    let title_line = Line::from(vec![
        Span::styled(title_prefix, if title_active { acc } else { dim }),
        Span::styled(title_val,    if title_active { fg  } else { dim }),
    ]);

    // ── Start time row ───────────────────────────────────────────────────────
    let start_prefix = if start_active { "▶ Start  " } else { "  Start  " };
    let start_line: Line = if start_active {
        Line::from(vec![
            Span::styled(start_prefix, acc),
            Span::styled(
                format!("{:02}", app.ui.event_start_h),
                if hour_focus { sel } else { fg },
            ),
            Span::styled(" : ", dim),
            Span::styled(
                format!("{:02}", app.ui.event_start_m),
                if !hour_focus { sel } else { fg },
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(start_prefix, dim),
            Span::styled(
                format!("{:02} : {:02}", app.ui.event_start_h, app.ui.event_start_m),
                dim,
            ),
        ])
    };

    // ── End time row ─────────────────────────────────────────────────────────
    let end_prefix = if end_active { "▶ End    " } else { "  End    " };
    let end_line: Line = if end_active {
        Line::from(vec![
            Span::styled(end_prefix, acc),
            Span::styled(
                format!("{:02}", app.ui.event_end_h),
                if hour_focus { sel } else { fg },
            ),
            Span::styled(" : ", dim),
            Span::styled(
                format!("{:02}", app.ui.event_end_m),
                if !hour_focus { sel } else { fg },
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(end_prefix, dim),
            Span::styled(
                format!("{:02} : {:02}", app.ui.event_end_h, app.ui.event_end_m),
                dim,
            ),
        ])
    };

    // ── Hint line ────────────────────────────────────────────────────────────
    let hint: Line = match step {
        EventFormStep::Title =>
            Line::from(Span::styled("  Enter: set time   Esc: cancel", dim)),
        EventFormStep::StartTime =>
            Line::from(Span::styled("  ↑↓ adjust   ←→ hour/min   Enter: set end", dim)),
        EventFormStep::EndTime =>
            Line::from(Span::styled("  ↑↓ adjust   ←→ hour/min   Enter: save", dim)),
    };

    // ── Step indicator ───────────────────────────────────────────────────────
    let step_num = match step {
        EventFormStep::Title     => "Step 1 / 3 — Title",
        EventFormStep::StartTime => "Step 2 / 3 — Start time",
        EventFormStep::EndTime   => "Step 3 / 3 — End time",
    };
    let step_line = Line::from(Span::styled(
        format!("  {step_num}"),
        Style::default().fg(t.muted()),
    ));

    let sep = Line::from(Span::styled(
        "─".repeat(inner.width.saturating_sub(2) as usize),
        dim,
    ));

    let lines: Vec<Line> = vec![
        Line::from(""),
        step_line,
        Line::from(""),
        title_line,
        Line::from(""),
        start_line,
        Line::from(""),
        end_line,
        Line::from(""),
        sep,
        Line::from(""),
        hint,
    ];

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(t.popup_bg())),
        inner,
    );
}

// ─── Simple text input popup (for tasks) ─────────────────────────────────────

fn draw_popup(f: &mut Frame, label: &str, value: &str, area: Rect, app: &App) {
    let t    = &app.theme;
    let rect = centered(60, 20, area);
    f.render_widget(Clear, rect);

    let title = Line::from(Span::styled(
        format!(" {label} "),
        Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
    ));
    let block = Block::default()
        .title(Title::from(title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_active()))
        .style(Style::default().bg(t.popup_bg()));

    f.render_widget(
        Paragraph::new(format!("{value}█")).block(block).style(Style::default().fg(t.fg())),
        rect,
    );
}

// ─── Help overlay ────────────────────────────────────────────────────────────

fn draw_help(f: &mut Frame, area: Rect, app: &App) {
    let t    = &app.theme;
    let rect = centered(68, 80, area);
    f.render_widget(Clear, rect);

    let title = Line::from(Span::styled(
        " Keyboard Shortcuts ",
        Style::default().fg(t.accent()).add_modifier(Modifier::BOLD),
    ));
    let block = Block::default()
        .title(Title::from(title))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_active()))
        .style(Style::default().bg(t.popup_bg()));

    let accent = Style::default().fg(t.accent()).add_modifier(Modifier::BOLD);
    let dim    = Style::default().fg(t.fg_dim());
    let lines  = vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", accent)),
        Line::from(Span::styled("  h/j/k/l  ←↓↑→     Move by day", dim)),
        Line::from(Span::styled("  [ / ]              Prev / Next month", dim)),
        Line::from(Span::styled("  t                  Jump to today", dim)),
        Line::from(Span::styled("  Tab                Cycle panels", dim)),
        Line::from(""),
        Line::from(Span::styled("  Events", accent)),
        Line::from(Span::styled("  n                  New event (3-step: title → start → end)", dim)),
        Line::from(Span::styled("    Enter              Advance to next step", dim)),
        Line::from(Span::styled("    ↑ / ↓              Adjust hour or minute", dim)),
        Line::from(Span::styled("    ← / →              Switch hour / minute field", dim)),
        Line::from(Span::styled("  d / Del            Delete event", dim)),
        Line::from(Span::styled("  Enter              Focus event list", dim)),
        Line::from(""),
        Line::from(Span::styled("  Tasks", accent)),
        Line::from(Span::styled("  N                  New task", dim)),
        Line::from(Span::styled("  Space              Toggle complete", dim)),
        Line::from(""),
        Line::from(Span::styled("  Sync (Google Calendar + Tasks)", accent)),
        Line::from(Span::styled("  Ctrl+s             Force sync now", dim)),
        Line::from(Span::styled("  Auto-sync every 5 minutes when configured", dim)),
        Line::from(""),
        Line::from(Span::styled("  General", accent)),
        Line::from(Span::styled("  ?                  Toggle help", dim)),
        Line::from(Span::styled("  Esc                Cancel / back", dim)),
        Line::from(Span::styled("  q                  Quit", dim)),
    ];

    f.render_widget(
        Paragraph::new(lines).block(block).style(Style::default().fg(t.fg()))
            .wrap(Wrap { trim: false }),
        rect,
    );
}

// ─── Utilities ────────────────────────────────────────────────────────────────

fn centered(pct_x: u16, pct_y: u16, r: Rect) -> Rect {
    let vert = Layout::default().direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ]).split(r);
    Layout::default().direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ]).split(vert[1])[1]
}

fn month_name(m: u32) -> &'static str {
    match m {
        1=>"January", 2=>"February", 3=>"March",    4=>"April",
        5=>"May",     6=>"June",     7=>"July",      8=>"August",
        9=>"September",10=>"October",11=>"November",12=>"December",
        _=>"???",
    }
}
