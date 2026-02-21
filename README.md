# LifeManager (`lm`)

> Riced TUI life manager — calendar, events, tasks, Google Calendar sync.
> Built with Rust + ratatui. Designed for Hyprland / Wayland.

```
╭─── February 2026 ──────────────────────────────────────────╮
│ Mo Tu We Th Fr Sa Su   ╭── Events — Monday, Feb 2 ────────╮│
│ ──────────────────      │ ● 09:00  Standup                 ││
│  2  3  4  5  6  7  8   │ ● 14:00  Team sync               ││
│  9 10 11 12 13 14 15   ╰──────────────────────────────────╯│
│ 16 17 18 19 20 21 22   ╭── Tasks ──────────────────────────╮│
│ 23 24 25 26 27 28      │ ○ Review PR #142                  ││
╰────────────────────────│ ✔ Update docs                     ││
                         ╰──────────────────────────────────╯│
 NORMAL  hjkl:nav  n:event  N:task  Tab:panels  ?:help  q:quit
```


## Inspiration

This project started from [lvsk-calendar](https://github.com/Gianluska/lvsk-calendar) — a lightweight, beautiful Bash terminal calendar built for Arch Linux and Hyprland. It nailed the philosophy: rounded box-drawing borders, vim-style navigation, hex-based theming, zero-config Hyprland floating window support. But it was entirely read-only — no events, no tasks, no sync.

LifeManager takes that same riced, minimal aesthetic and builds a complete personal organisation tool on top of it: one that can fully replace a GUI calendar app while living entirely in the terminal.

---

## Tech Stack

| Layer | Technology | Why |
|---|---|---|
| **Language** | Rust | Sub-millisecond startup, ~5 MB resident memory, zero GC pauses — ideal for a floating TUI |
| **TUI framework** | [ratatui](https://github.com/ratatui-org/ratatui) `0.28` | Most mature Rust TUI lib; stateless immediate-mode rendering |
| **Terminal backend** | [crossterm](https://github.com/crossterm-rs/crossterm) `0.28` | Raw mode, key events, Wayland-compatible |
| **Async runtime** | [tokio](https://tokio.rs) `1` | Drives the background sync worker and HTTP client independently of the TUI thread |
| **Database** | SQLite via [sqlx](https://github.com/launchbadge/sqlx) `0.8` | Offline-first local storage; async queries; no external server needed |
| **HTTP client** | [reqwest](https://github.com/seanmonstar/reqwest) `0.12` | Google Calendar and Tasks REST API calls |
| **Serialization** | [serde](https://serde.rs) + serde_json | JSON for API payloads; TOML for config and theme files |
| **Config / Theme** | [toml](https://github.com/toml-rs/toml) `0.8` | Human-editable hex colour files |
| **Date / Time** | [chrono](https://github.com/chronotope/chrono) `0.4` | Calendar math, RFC 3339 parsing, UTC ↔ local time |
| **Unique IDs** | [uuid](https://github.com/uuid-rs/uuid) `v4` | Collision-free local record IDs before remote sync assigns a server ID |
| **Logging** | [tracing](https://github.com/tokio-rs/tracing) + [tracing-appender](https://docs.rs/tracing-appender) | File-only logs — stdout/stderr would corrupt the TUI |
| **Config dirs** | [dirs](https://github.com/dirs-dev/dirs-rs) | XDG-compliant paths (`~/.config`, `~/.local/share`) |
| **Browser launch** | [open](https://github.com/Byron/open-rs) `5` | Opens the OAuth2 consent URL in the default browser |

### Why Rust over Go or Bash?

The original lvsk-calendar is pure Bash — no compilation, instant startup, perfect for a read-only display widget. Once you add a reactive 50 ms-tick TUI, an async sync worker, a database, and an HTTP client, Bash becomes the wrong tool. The real choice was between Rust and Go:

**Go** has solid TUI support via `bubbletea` and excellent ergonomics, but its garbage collector introduces occasional pauses that are visible in fast TUI loops, and binaries are roughly 2× larger. **Rust** compiles to a ~4 MB stripped binary, uses ~5 MB RAM, has zero GC, and `ratatui` is more battle-tested than any Go TUI library. The borrow checker adds friction during development but eliminates an entire class of runtime bugs — important for a long-running process that syncs personal data bidirectionally with a remote API.

---


## Quick Start

```bash
# 1. Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Build
cargo build --release

# 3. (Optional) Google Calendar sync
#    a. Go to https://console.cloud.google.com
#    b. Create project → enable "Google Calendar API"
#    c. Create OAuth2 "Desktop app" credential
#    d. Copy client_id + client_secret into config:
cp config.example.toml ~/.config/lifemanager/config.toml
$EDITOR ~/.config/lifemanager/config.toml

# 4. Run
./target/release/lm
# Or install:
./install.sh
```

## Keybindings

| Key | Action |
|-----|--------|
| `h/j/k/l` or arrows | Navigate days |
| `[` / `]` | Prev / Next month |
| `t` | Jump to today |
| `n` | New event on selected day |
| `N` | New task |
| `Space` | Toggle task complete |
| `d` / `Del` | Delete selected item |
| `Tab` | Cycle panel focus |
| `Ctrl+s` | Force sync |
| `?` | Help |
| `Esc` | Cancel |
| `q` | Quit |

## Themes

Edit `~/.config/lifemanager/theme.toml` (auto-generated on first run).
Change any hex value — supports Catppuccin Mocha (default), Nord, Gruvbox.

## Hyprland integration

```bash
# Add to hyprland.conf:
bind = $mainMod, C, exec, foot --title LifeManager lm

# Auto-float the window:
windowrulev = float, title:^(LifeManager)$
windowrulev = size 900 600, title:^(LifeManager)$
windowrulev = center, title:^(LifeManager)$
```

## Images

<img width="1083" height="420" alt="image" src="https://github.com/user-attachments/assets/34178744-59e8-46c4-828b-8f48d1f6e5a1" />
<img width="1086" height="421" alt="image" src="https://github.com/user-attachments/assets/937e5cf6-4562-4193-a7b7-5f3bb9092709" />
<img width="1080" height="855" alt="image" src="https://github.com/user-attachments/assets/162559a8-fda7-4f0b-803b-afb3fa401166" />



## Themes
- Nord (default)
<img width="1080" height="855" alt="image" src="https://github.com/user-attachments/assets/3eb82924-bc0a-4e3f-b725-4898c29f289b" />

# Gruvbox
<img width="1082" height="857" alt="image" src="https://github.com/user-attachments/assets/483baf86-fc44-4f1f-a131-7b2a2ad4849f" />

# Tokyo night
<img width="1081" height="855" alt="image" src="https://github.com/user-attachments/assets/d83642e2-a8a5-4d55-bf33-abdc2ac6e8a8" />

# Dracula
<img width="1084" height="858" alt="image" src="https://github.com/user-attachments/assets/b98cbefd-da5c-409a-b554-b4b39623a32a" />

# Cyberpunk
<img width="1082" height="851" alt="image" src="https://github.com/user-attachments/assets/8e7952ba-1f7f-4fd0-ba61-068cffc9951e" />

# hacker
<img width="1081" height="849" alt="image" src="https://github.com/user-attachments/assets/9a9ed49a-6254-4357-8007-9d920c362250" />

# vietnam (my favorites)
<img width="1082" height="850" alt="image" src="https://github.com/user-attachments/assets/c464e87d-b43c-4d54-aa51-2fe76267d527" />

# catppuccin-mocha
<img width="1079" height="856" alt="image" src="https://github.com/user-attachments/assets/c228ee69-586a-4567-a72f-f509d14a2605" />



## Roadmap

- [x] Month calendar with vim navigation
- [x] Event CRUD with 3-step time-picker form
- [x] Task CRUD with completion toggle
- [x] SQLite offline-first storage
- [x] Hex-based theme engine (Catppuccin, Nord, Gruvbox)
- [x] Google Calendar OAuth2 + bi-directional sync ✅
- [x] Google Tasks bi-directional sync ✅
- [x] Background Tokio sync worker (auto every 5 min)
- [x] `lm auth google` CLI command
- [x] `lm sync` headless sync command
- [x] Conflict resolution — dirty flag preserves local edits
- [ ] Week view (7-column hourly grid)
- [ ] Day view (hourly timeline)
- [ ] Agenda view (flat scrollable list)
- [ ] Edit existing events (full form, not just create)
- [ ] Apple Calendar / iCloud CalDAV sync
- [ ] Fuzzy search across events and tasks (`/`)
- [ ] Recurring event display
- [ ] Wayland notifications on event start
- [ ] AUR package
- [ ] Nix flake

---

## Acknowledgements

- [lvsk-calendar](https://github.com/Gianluska/lvsk-calendar) by Gianluska — the aesthetic that started this project
- [ratatui](https://github.com/ratatui-org/ratatui) — the TUI framework that made it possible
- Catppuccin, Nord, and Gruvbox theme communities

---

## License

MIT

