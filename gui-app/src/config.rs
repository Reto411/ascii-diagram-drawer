//! Application options (FR13 / CR-005): user preferences persisted across sessions.
//!
//! The options live in a JSON file in the platform config directory
//! ([`default_config_path`]). Loading is never fatal — a missing, unreadable or
//! corrupt file yields the defaults; saving is best-effort and its failure is
//! reported through the status bar, not panicked. Unknown fields are ignored and
//! absent ones default, so files written by other app versions still load.

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The UI theme choices (FR13). `Native` is the default: inherit the system /
/// window manager's appearance — eframe 0.37 delivers it natively on every target
/// (winit's OS query, `prefers-color-scheme` on the web), and egui falls back to
/// dark where it is unknown, exactly the fallback CR-005 allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Mainly white.
    Light,
    /// Mainly dark.
    Dark,
    /// Inherited from the system / window manager (default).
    #[default]
    Native,
}

impl Theme {
    /// The egui theme preference this choice maps to. `Native` is
    /// [`egui::ThemePreference::System`]: egui follows the system appearance that
    /// eframe feeds through `RawInput::system_theme`, and resolves to its dark
    /// fallback where the appearance is unknown (CR-005).
    #[must_use]
    pub fn preference(self) -> egui::ThemePreference {
        match self {
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
            Self::Native => egui::ThemePreference::System,
        }
    }

    /// The label shown in the Options menu.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Native => "Native (system)",
        }
    }

    /// The status-bar name ("Light" / "Dark" / "Native").
    #[must_use]
    pub fn short(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Native => "native",
        }
    }
}

/// The persisted user options. Extensible: future settings add fields here and to
/// the Options menu without a format change (unknown/absent fields never break).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// The UI theme choice (FR13).
    #[serde(default)]
    pub theme: Theme,
}

impl AppConfig {
    /// Loads options from `path`. `None` (no platform config location — e.g. wasm)
    /// or a missing/corrupt file yields the defaults; loading is never fatal.
    #[must_use]
    pub fn load(path: Option<&Path>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Writes the options to `path`, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json + "\n")
    }
}

/// The default options file: the platform config directory
/// (`$XDG_CONFIG_HOME`/`~/.config`, `%APPDATA%`, `~/Library/Application Support`)
/// under `ascii-diagram-drawer/config.json`. `None` where the platform exposes no
/// config directory (e.g. wasm) — then options are not persisted.
#[must_use]
pub fn default_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "ascii-diagram-drawer")
        .map(|dirs| dirs.config_dir().join("config.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_native() {
        assert_eq!(AppConfig::default().theme, Theme::Native);
        assert_eq!(Theme::default(), Theme::Native);
    }

    #[test]
    fn all_themes_round_trip_through_json() {
        for theme in [Theme::Light, Theme::Dark, Theme::Native] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("config.json");
            AppConfig { theme }.save(&path).unwrap();
            assert_eq!(AppConfig::load(Some(&path)).theme, theme);
        }
    }

    #[test]
    fn missing_file_loads_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope/config.json");
        assert_eq!(AppConfig::load(Some(&path)), AppConfig::default());
        assert_eq!(AppConfig::load(None), AppConfig::default());
    }

    #[test]
    fn corrupt_file_loads_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "this is not json {").unwrap();
        assert_eq!(AppConfig::load(Some(&path)), AppConfig::default());
    }

    #[test]
    fn unknown_theme_value_falls_back_to_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"theme": "neon"}"#).unwrap();
        assert_eq!(AppConfig::load(Some(&path)).theme, Theme::Native);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"theme": "dark", "zoom": 3}"#).unwrap();
        assert_eq!(AppConfig::load(Some(&path)).theme, Theme::Dark);
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/config.json");
        AppConfig {
            theme: Theme::Light,
        }
        .save(&path)
        .unwrap();
        assert!(path.exists());
    }

    #[test]
    fn theme_choices_map_to_egui_preferences() {
        assert_eq!(Theme::Light.preference(), egui::ThemePreference::Light);
        assert_eq!(Theme::Dark.preference(), egui::ThemePreference::Dark);
        assert_eq!(Theme::Native.preference(), egui::ThemePreference::System);
    }

    #[test]
    fn native_falls_back_to_dark_without_a_system_theme() {
        // CR-005: where the system appearance cannot be detected, native = dark.
        let ctx = egui::Context::default();
        ctx.set_theme(Theme::Native.preference());
        assert_eq!(ctx.theme(), egui::Theme::Dark);
    }
}
