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
windowrulev2 = float, title:^(LifeManager)$
windowrulev2 = size 900 600, title:^(LifeManager)$
windowrulev2 = center, title:^(LifeManager)$
```
