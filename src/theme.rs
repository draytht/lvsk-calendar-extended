use anyhow::Result;
use ratatui::style::Color;
use ratatui::widgets::BorderType;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn hex_to_color(hex: &str) -> Color {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 { return Color::Reset; }
    let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
    Color::Rgb(r, g, b)
}

fn default_holiday_dot()   -> String { "#f9e2af".to_owned() }
fn default_border_style()  -> String { "rounded".to_owned() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    // Backgrounds
    pub bg_primary: String, pub bg_secondary: String, pub bg_popup: String,
    // Borders
    pub border_normal: String, pub border_focused: String, pub border_selected: String,
    // Text
    pub text_primary: String, pub text_secondary: String,
    pub text_muted: String, pub text_accent: String,
    // Highlights
    pub today_bg: String, pub today_fg: String,
    pub selected_bg: String, pub selected_fg: String,
    // Special
    pub event_dot: String, pub weekend_fg: String,
    pub success: String, pub warning: String, pub error: String,
    /// Star color for holiday markers on the calendar.
    #[serde(default = "default_holiday_dot")]
    pub holiday_dot: String,
    /// Box-drawing chars (kept for custom TOML themes).
    pub char_h: String, pub char_v: String,
    pub char_tl: String, pub char_tr: String,
    pub char_bl: String, pub char_br: String,
    /// Border style: "rounded" | "double" | "thick" | "plain"
    #[serde(default = "default_border_style")]
    pub border_style: String,
}

impl ThemeConfig {
    // ── Color accessors ───────────────────────────────────────────────────────
    pub fn bg(&self)            -> Color { hex_to_color(&self.bg_primary) }
    pub fn bg2(&self)           -> Color { hex_to_color(&self.bg_secondary) }
    pub fn popup_bg(&self)      -> Color { hex_to_color(&self.bg_popup) }
    pub fn border(&self)        -> Color { hex_to_color(&self.border_normal) }
    pub fn border_active(&self) -> Color { hex_to_color(&self.border_focused) }
    pub fn fg(&self)            -> Color { hex_to_color(&self.text_primary) }
    pub fn fg_dim(&self)        -> Color { hex_to_color(&self.text_muted) }
    pub fn accent(&self)        -> Color { hex_to_color(&self.text_accent) }
    pub fn event_color(&self)   -> Color { hex_to_color(&self.event_dot) }
    pub fn weekend_color(&self) -> Color { hex_to_color(&self.weekend_fg) }
    pub fn muted(&self)         -> Color { hex_to_color(&self.text_muted) }
    pub fn holiday(&self)       -> Color { hex_to_color(&self.holiday_dot) }
    pub fn error(&self)         -> Color { hex_to_color(&self.error) }

    pub fn today_highlight(&self)    -> (Color, Color) {
        (hex_to_color(&self.today_bg), hex_to_color(&self.today_fg))
    }
    pub fn selected_highlight(&self) -> (Color, Color) {
        (hex_to_color(&self.selected_bg), hex_to_color(&self.selected_fg))
    }

    pub fn border_type(&self) -> BorderType {
        match self.border_style.as_str() {
            "double" => BorderType::Double,
            "thick"  => BorderType::Thick,
            "plain"  => BorderType::Plain,
            _        => BorderType::Rounded,
        }
    }

    // ── Persistence ───────────────────────────────────────────────────────────
    pub fn load() -> Result<Self> {
        let path = config_dir().join("theme.toml");
        if path.exists() {
            Ok(toml::from_str(&std::fs::read_to_string(&path)?)?)
        } else {
            let t = ThemeConfig::default();
            t.save()?;
            Ok(t)
        }
    }

    pub fn save(&self) -> Result<()> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("theme.toml"), toml::to_string_pretty(self)?)?;
        Ok(())
    }

    // ── Theme catalogue ───────────────────────────────────────────────────────
    pub fn all_themes() -> Vec<ThemeConfig> {
        vec![
            ThemeConfig::default(),    // Catppuccin Mocha
            ThemeConfig::nord(),
            ThemeConfig::gruvbox(),
            ThemeConfig::tokyo_night(),
            ThemeConfig::dracula(),
            ThemeConfig::cyberpunk(),
            ThemeConfig::hacker(),
            ThemeConfig::vietnam(),
        ]
    }

    // ── Built-in themes ───────────────────────────────────────────────────────

    pub fn nord() -> Self { Self {
        name: "nord".into(),
        bg_primary: "#2e3440".into(), bg_secondary: "#3b4252".into(), bg_popup: "#434c5e".into(),
        border_normal: "#4c566a".into(), border_focused: "#88c0d0".into(), border_selected: "#81a1c1".into(),
        text_primary: "#eceff4".into(), text_secondary: "#e5e9f0".into(),
        text_muted: "#4c566a".into(), text_accent: "#88c0d0".into(),
        today_bg: "#88c0d0".into(), today_fg: "#2e3440".into(),
        selected_bg: "#81a1c1".into(), selected_fg: "#2e3440".into(),
        event_dot: "#a3be8c".into(), weekend_fg: "#bf616a".into(),
        success: "#a3be8c".into(), warning: "#ebcb8b".into(), error: "#bf616a".into(),
        holiday_dot: "#ebcb8b".into(),
        char_h: "─".into(), char_v: "│".into(),
        char_tl: "╭".into(), char_tr: "╮".into(), char_bl: "╰".into(), char_br: "╯".into(),
        border_style: "rounded".into(),
    }}

    pub fn gruvbox() -> Self { Self {
        name: "gruvbox".into(),
        bg_primary: "#282828".into(), bg_secondary: "#1d2021".into(), bg_popup: "#3c3836".into(),
        border_normal: "#504945".into(), border_focused: "#d79921".into(), border_selected: "#689d6a".into(),
        text_primary: "#ebdbb2".into(), text_secondary: "#d5c4a1".into(),
        text_muted: "#7c6f64".into(), text_accent: "#d79921".into(),
        today_bg: "#d79921".into(), today_fg: "#282828".into(),
        selected_bg: "#689d6a".into(), selected_fg: "#282828".into(),
        event_dot: "#b8bb26".into(), weekend_fg: "#fb4934".into(),
        success: "#b8bb26".into(), warning: "#fabd2f".into(), error: "#fb4934".into(),
        holiday_dot: "#fabd2f".into(),
        char_h: "─".into(), char_v: "│".into(),
        char_tl: "╭".into(), char_tr: "╮".into(), char_bl: "╰".into(), char_br: "╯".into(),
        border_style: "rounded".into(),
    }}

    pub fn tokyo_night() -> Self { Self {
        name: "tokyo-night".into(),
        bg_primary: "#1a1b26".into(), bg_secondary: "#16161e".into(), bg_popup: "#24283b".into(),
        border_normal: "#3d4166".into(), border_focused: "#7aa2f7".into(), border_selected: "#bb9af7".into(),
        text_primary: "#c0caf5".into(), text_secondary: "#a9b1d6".into(),
        text_muted: "#565f89".into(), text_accent: "#7aa2f7".into(),
        today_bg: "#bb9af7".into(), today_fg: "#1a1b26".into(),
        selected_bg: "#7aa2f7".into(), selected_fg: "#1a1b26".into(),
        event_dot: "#9ece6a".into(), weekend_fg: "#f7768e".into(),
        success: "#9ece6a".into(), warning: "#e0af68".into(), error: "#f7768e".into(),
        holiday_dot: "#e0af68".into(),
        char_h: "─".into(), char_v: "│".into(),
        char_tl: "╭".into(), char_tr: "╮".into(), char_bl: "╰".into(), char_br: "╯".into(),
        border_style: "rounded".into(),
    }}

    pub fn dracula() -> Self { Self {
        name: "dracula".into(),
        bg_primary: "#282a36".into(), bg_secondary: "#21222c".into(), bg_popup: "#44475a".into(),
        border_normal: "#6272a4".into(), border_focused: "#bd93f9".into(), border_selected: "#ff79c6".into(),
        text_primary: "#f8f8f2".into(), text_secondary: "#e2e2e2".into(),
        text_muted: "#6272a4".into(), text_accent: "#bd93f9".into(),
        today_bg: "#50fa7b".into(), today_fg: "#282a36".into(),
        selected_bg: "#ff79c6".into(), selected_fg: "#282a36".into(),
        event_dot: "#50fa7b".into(), weekend_fg: "#ff5555".into(),
        success: "#50fa7b".into(), warning: "#f1fa8c".into(), error: "#ff5555".into(),
        holiday_dot: "#f1fa8c".into(),
        char_h: "─".into(), char_v: "│".into(),
        char_tl: "╭".into(), char_tr: "╮".into(), char_bl: "╰".into(), char_br: "╯".into(),
        border_style: "rounded".into(),
    }}

    /// Neon cyberpunk — electric pink & cyan on deep purple-black.
    pub fn cyberpunk() -> Self { Self {
        name: "cyberpunk".into(),
        bg_primary: "#0a0014".into(), bg_secondary: "#120028".into(), bg_popup: "#1e003c".into(),
        border_normal: "#3d005c".into(), border_focused: "#ff00ff".into(), border_selected: "#00ffff".into(),
        text_primary: "#e8d5ff".into(), text_secondary: "#b080ff".into(),
        text_muted: "#5c2080".into(), text_accent: "#00ffff".into(),
        today_bg: "#ff00ff".into(), today_fg: "#000000".into(),
        selected_bg: "#00ffff".into(), selected_fg: "#000000".into(),
        event_dot: "#ff6600".into(), weekend_fg: "#ff00ff".into(),
        success: "#00ff88".into(), warning: "#ffaa00".into(), error: "#ff0044".into(),
        holiday_dot: "#ffff00".into(),
        char_h: "═".into(), char_v: "║".into(),
        char_tl: "╔".into(), char_tr: "╗".into(), char_bl: "╚".into(), char_br: "╝".into(),
        border_style: "thick".into(),
    }}

    /// Matrix / hacker — phosphor green on pure black, double-line borders.
    pub fn hacker() -> Self { Self {
        name: "hacker".into(),
        bg_primary: "#000000".into(), bg_secondary: "#001100".into(), bg_popup: "#001a00".into(),
        border_normal: "#003300".into(), border_focused: "#00ff41".into(), border_selected: "#00cc33".into(),
        text_primary: "#00cc33".into(), text_secondary: "#009922".into(),
        text_muted: "#004411".into(), text_accent: "#00ff41".into(),
        today_bg: "#00ff41".into(), today_fg: "#000000".into(),
        selected_bg: "#003300".into(), selected_fg: "#00ff41".into(),
        event_dot: "#ff3300".into(), weekend_fg: "#00aa22".into(),
        success: "#00ff41".into(), warning: "#ffff00".into(), error: "#ff0000".into(),
        holiday_dot: "#ffff00".into(),
        char_h: "═".into(), char_v: "║".into(),
        char_tl: "╔".into(), char_tr: "╗".into(), char_bl: "╚".into(), char_br: "╝".into(),
        border_style: "double".into(),
    }}

    /// Vietnamese flag palette — crimson red & golden yellow.
    pub fn vietnam() -> Self { Self {
        name: "vietnam".into(),
        bg_primary: "#1a0000".into(), bg_secondary: "#110000".into(), bg_popup: "#2a0000".into(),
        border_normal: "#5c0000".into(), border_focused: "#ffd700".into(), border_selected: "#ff6600".into(),
        text_primary: "#ffe8cc".into(), text_secondary: "#ffcc88".into(),
        text_muted: "#7a3300".into(), text_accent: "#ffd700".into(),
        today_bg: "#cc0000".into(), today_fg: "#ffd700".into(),
        selected_bg: "#ffd700".into(), selected_fg: "#1a0000".into(),
        event_dot: "#ff6600".into(), weekend_fg: "#ff4444".into(),
        success: "#ffd700".into(), warning: "#ff8800".into(), error: "#ff0000".into(),
        holiday_dot: "#ffd700".into(),
        char_h: "─".into(), char_v: "│".into(),
        char_tl: "╭".into(), char_tr: "╮".into(), char_bl: "╰".into(), char_br: "╯".into(),
        border_style: "rounded".into(),
    }}
}

impl Default for ThemeConfig {
    fn default() -> Self { Self {
        name: "catppuccin-mocha".into(),
        bg_primary: "#1e1e2e".into(), bg_secondary: "#181825".into(), bg_popup: "#313244".into(),
        border_normal: "#45475a".into(), border_focused: "#89b4fa".into(), border_selected: "#cba6f7".into(),
        text_primary: "#cdd6f4".into(), text_secondary: "#bac2de".into(),
        text_muted: "#6c7086".into(), text_accent: "#89b4fa".into(),
        today_bg: "#cba6f7".into(), today_fg: "#1e1e2e".into(),
        selected_bg: "#89b4fa".into(), selected_fg: "#1e1e2e".into(),
        event_dot: "#a6e3a1".into(), weekend_fg: "#f38ba8".into(),
        success: "#a6e3a1".into(), warning: "#f9e2af".into(), error: "#f38ba8".into(),
        holiday_dot: "#f9e2af".into(),
        char_h: "─".into(), char_v: "│".into(),
        char_tl: "╭".into(), char_tr: "╮".into(), char_bl: "╰".into(), char_br: "╯".into(),
        border_style: "rounded".into(),
    }}
}

fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("lifemanager")
}
