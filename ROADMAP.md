# LifeManager — Project Roadmap

> A riced TUI Life Manager for Hyprland/Wayland, inspired by lvsk-calendar.
> Built in **Rust** with `ratatui`, `tokio`, `sqlx`, and `reqwest`.

---

## Language Choice: Why Rust?

| Criterion | Rust | Go |
|---|---|---|
| **Wayland/Hyprland perf** | ✅ Zero-cost abstractions, sub-ms startup | ✅ Fast but GC pauses |
| **TUI ecosystem** | ✅ `ratatui` (most mature TUI lib) | ✅ `bubbletea` (excellent) |
| **Async** | ✅ `tokio` (industry standard) | ✅ native goroutines |
| **SQLite** | ✅ `sqlx` with async + compile-time checks | ✅ `modernc.org/sqlite` |
| **Binary size** | ✅ ~4MB stripped | ✅ ~8MB |
| **Memory** | ✅ ~5MB resident | ~15MB with GC overhead |
| **Compile time** | ⚠️ Slower (worth it for runtime) | ✅ Fast |

**Verdict: Rust.** For a TUI that runs in a Hyprland floating window, startup latency and memory footprint matter. Rust wins both, and `ratatui` is battle-tested for exactly this use case.

---

## Phase 1: Foundation (Week 1–2)

### Goals
- Working TUI skeleton with calendar navigation
- SQLite database setup
- Local event/task CRUD

### Deliverables
- [x] `Cargo.toml` with all dependencies
- [x] `ratatui` main loop with crossterm backend
- [x] Calendar grid widget (month view, vim navigation)
- [x] Event list panel
- [x] Task list panel
- [x] SQLite schema + `sqlx` integration
- [x] Create/delete events and tasks from TUI
- [x] Theme engine (hex → ratatui Color)

### Key Files
```
src/
├── main.rs         — entry point, tokio runtime
├── app/mod.rs      — state machine + input handling
├── ui/mod.rs       — ratatui rendering
├── db/mod.rs       — SQLite CRUD layer
├── calendar/mod.rs — date math utilities
├── tasks/mod.rs    — task business logic
└── theme.rs        — theme engine + presets
```

---

## Phase 2: Google Calendar Sync (Week 3–4)

### Goals
- Full OAuth2 PKCE flow for Google
- Pull events from Google Calendar
- Push local changes back up
- Background async sync worker (every 5 min)

### Deliverables
- [x] `sync/google.rs` — OAuth2 + REST API client
- [x] `sync/worker.rs` — background Tokio task
- [ ] `lm auth google` CLI subcommand (opens browser, captures callback)
- [ ] Incremental sync using `syncToken` from Google API
- [ ] Conflict resolution (last-write-wins with `etag` check)
- [ ] Pull event color/calendar associations

### Google Cloud Console Setup
1. Go to `console.cloud.google.com`
2. Create a new project → Enable **Google Calendar API** + **Tasks API**
3. Create OAuth2 credentials → type: **Desktop app**
4. Copy `client_id` + `client_secret` into `~/.config/lifemanager/config.toml`
5. Add `http://localhost:8085/callback` to authorized redirect URIs
6. Run `lm auth google` — browser opens → authorize → token saved

---

## Phase 3: Apple Calendar / CalDAV (Week 5)

### Goals
- Bi-directional sync with iCloud Calendar via CalDAV
- Support for any CalDAV server (Nextcloud, Fastmail, etc.)

### Key Libraries
- `reqwest` for HTTP
- Parse iCalendar (`.ics`) format manually or via `icalendar` crate

### Deliverables
- [ ] `sync/caldav.rs` — CalDAV REPORT / PUT / DELETE
- [ ] iCal parse/emit for events
- [ ] App-specific password support for iCloud
- [ ] `lm auth caldav` subcommand

### iCloud CalDAV Config
```toml
[caldav]
url      = "https://caldav.icloud.com"
username = "your@icloud.com"
password = "xxxx-xxxx-xxxx-xxxx"  # App-specific password
```

---

## Phase 4: Advanced TUI Features (Week 6–7)

### Goals
- Multiple views (month, week, day, agenda)
- Event detail editor (full form with time picker)
- Fuzzy search across events and tasks
- Recurring event display

### Deliverables
- [ ] **Week view** — 7-column grid with time slots
- [ ] **Day view** — hourly timeline
- [ ] **Agenda view** — flat scrollable list
- [ ] **Event editor popup** — title, time, description, color
- [ ] **Fuzzy search** — `/` to search, powered by `nucleo` or `fuzzy-matcher`
- [ ] **Recurring events** — display expanded instances (rrule parsing)
- [ ] **Notifications** — `notify-send` via Wayland on event start

---

## Phase 5: Polish & Ricing (Week 8)

### Goals
- Complete theme system matching lvsk-calendar's aesthetic
- Hyprland integration (window rules, Waybar widget)
- AUR package

### Deliverables
- [ ] 5 built-in themes: Catppuccin Mocha, Nord, Gruvbox, Dracula, Kanagawa
- [ ] Custom border character sets (from `~/.config/lifemanager/theme.toml`)
- [ ] Hyprland launcher script (floating window, auto-size)
- [ ] Waybar module config
- [ ] `PKGBUILD` for AUR
- [ ] Nix flake

### Hyprland Launcher Script
```bash
#!/bin/bash
# lm-launcher
hyprctl keyword windowrule "float,title:LifeManager" 2>/dev/null
hyprctl keyword windowrule "size 900 600,title:LifeManager" 2>/dev/null
hyprctl keyword windowrule "center,title:LifeManager" 2>/dev/null

exec foot --title "LifeManager" lm
```

### Waybar Module
```json
"custom/lifemanager": {
  "format": " 󰃭 ",
  "on-click": "lm-launcher",
  "tooltip-format": "Open LifeManager"
}
```

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────┐
│                   TUI Thread (main)                  │
│  ┌──────────┐  ┌────────────┐  ┌─────────────────┐  │
│  │ Calendar │  │ Event List │  │   Task List     │  │
│  │  Widget  │  │  Widget    │  │   Widget        │  │
│  └──────────┘  └────────────┘  └─────────────────┘  │
│         │              │                │            │
│         └──────────────┴────────────────┘            │
│                        │                             │
│                   App State                          │
│                        │                             │
│              ┌──────────────────┐                    │
│              │  Database Layer  │                    │
│              │    (sqlx/SQLite) │                    │
│              └──────────────────┘                    │
└─────────────────────┬───────────────────────────────┘
                      │ mpsc channels
┌─────────────────────▼───────────────────────────────┐
│              Sync Worker (Tokio task)                │
│                                                      │
│  ┌──────────────────┐   ┌────────────────────────┐  │
│  │ Google Calendar  │   │  Apple CalDAV Client   │  │
│  │ OAuth2 + REST    │   │  (iCal/PROPFIND)       │  │
│  └──────────────────┘   └────────────────────────┘  │
│           │                          │               │
│   Google API v3                CalDAV Server         │
└─────────────────────────────────────────────────────┘
```

---

## Data Flow: Sync Cycle

```
Local create → dirty=true → DB
     ↓ (every 5min or Ctrl+S)
SyncWorker reads dirty events
     ↓
Push to Google (POST/PUT) → get remote ID + etag
     ↓
mark_event_clean(id, sync_id, etag)
     ↓
Pull from Google (list events with syncToken)
     ↓
Upsert into local DB (if etag changed)
     ↓
UI refreshes from DB
```

---

## Config Reference

```toml
# ~/.config/lifemanager/config.toml

[google]
client_id     = "…"
client_secret = "…"
calendar_ids  = ["primary", "your_other_cal_id"]

[caldav]
url      = "https://caldav.icloud.com"
username = "you@icloud.com"
password = "app-specific-password"

[sync]
interval_seconds = 300
auto_sync = true

# Theme is in ~/.config/lifemanager/theme.toml
# Edit hex values to fully customize colors
```

---

## Quick Start

```bash
# Clone and build
git clone https://github.com/you/lifemanager
cd lifemanager
cargo build --release

# Copy config
mkdir -p ~/.config/lifemanager
cp config.example.toml ~/.config/lifemanager/config.toml
# Edit with your Google client_id + client_secret

# Authenticate
./target/release/lm auth google
# → opens browser, authorize, token saved automatically

# Run
./target/release/lm
```

---

## Keybindings Reference

| Key | Action |
|-----|--------|
| `h/j/k/l` or `←↓↑→` | Navigate days |
| `[` / `]` | Previous / Next month |
| `t` | Jump to today |
| `n` | New event on selected day |
| `N` | New task |
| `d` / `Del` | Delete selected item |
| `Space` | Toggle task complete |
| `Tab` | Switch panel focus |
| `Ctrl+s` | Force sync now |
| `?` | Help |
| `Esc` | Cancel / back |
| `q` | Quit |
