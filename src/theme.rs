use anyhow::Result;
use ratatui::style::Color;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub name: String,
    pub bg_primary: String, pub bg_secondary: String, pub bg_popup: String,
    pub border_normal: String, pub border_focused: String, pub border_selected: String,
    pub text_primary: String, pub text_secondary: String,
    pub text_muted: String, pub text_accent: String,
    pub today_bg: String, pub today_fg: String,
    pub selected_bg: String, pub selected_fg: String,
    pub event_dot: String, pub weekend_fg: String,
    pub success: String, pub warning: String, pub error: String,
    pub char_h: String, pub char_v: String,
    pub char_tl: String, pub char_tr: String,
    pub char_bl: String, pub char_br: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "catppuccin-mocha".into(),
            bg_primary: "#1e1e2e".into(), bg_secondary: "#181825".into(), bg_popup: "#313244".into(),
            border_normal: "#45475a".into(), border_focused: "#89b4fa".into(), border_selected: "#cba6f7".into(),
            text_primary: "#cdd6f4".into(), text_secondary: "#bac2de".into(),
            text_muted: "#6c7086".into(), text_accent: "#89b4fa".into(),
            today_bg: "#cba6f7".into(), today_fg: "#1e1e2e".into(),
            selected_bg: "#89b4fa".into(), selected_fg: "#1e1e2e".into(),
            event_dot: "#a6e3a1".into(), weekend_fg: "#f38ba8".into(),
            success: "#a6e3a1".into(), warning: "#f9e2af".into(), error: "#f38ba8".into(),
            char_h: "─".into(), char_v: "│".into(),
            char_tl: "╭".into(), char_tr: "╮".into(),
            char_bl: "╰".into(), char_br: "╯".into(),
        }
    }
}

impl ThemeConfig {
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
        char_h: "─".into(), char_v: "│".into(),
        char_tl: "╭".into(), char_tr: "╮".into(), char_bl: "╰".into(), char_br: "╯".into(),
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
        char_h: "─".into(), char_v: "│".into(),
        char_tl: "╭".into(), char_tr: "╮".into(), char_bl: "╰".into(), char_br: "╯".into(),
    }}

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

    pub fn today_highlight(&self)    -> (Color, Color) {
        (hex_to_color(&self.today_bg), hex_to_color(&self.today_fg))
    }
    pub fn selected_highlight(&self) -> (Color, Color) {
        (hex_to_color(&self.selected_bg), hex_to_color(&self.selected_fg))
    }
}

fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("lifemanager")
}
