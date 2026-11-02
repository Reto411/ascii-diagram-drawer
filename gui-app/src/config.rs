//! Application options (FR13 / CR-005): user preferences persisted across sessions.
//!
//! The options live in a JSON file in the platform config directory
//! ([`default_config_path`]). Loading is never fatal — a missing, unreadable or
//! corrupt file yields the defaults; saving is best-effort and its failure is
//! reported through the status bar, not panicked. Unknown fields are ignored and
//! absent ones default, so files written by other app versions still load.

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

/// Most recent colors first; the strip shows at most this many (FR15 / CR-009 AC6).
pub const MAX_RECENT_COLORS: usize = 16;

/// The persisted user options. Extensible: future settings add fields here and to
/// the Options menu without a format change (unknown/absent fields never break).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// The UI theme choice (FR13).
    #[serde(default)]
    pub theme: Theme,
    /// Recently used diagram colors as lowercase `#rrggbb` strings, newest first
    /// (FR15 / CR-009 AC6). Absent in pre-FR15 files → empty strip.
    #[serde(default)]
    pub recent_colors: Vec<String>,
    /// Sidebar collapsing-header open states keyed by "group" or "group/subcategory"
    /// (FR6 / CR-011 AC3). A missing key means *expanded* — the first-use default —
    /// so pre-feature files load with everything open.
    #[serde(default)]
    pub sidebar_expanded: BTreeMap<String, bool>,
}

impl AppConfig {
    /// Records `hex` (`#rrggbb`) as the most recently used color: deduplicated, kept
    /// newest-first, capped at [`MAX_RECENT_COLORS`].
    pub fn record_recent_color(&mut self, hex: &str) {
        self.recent_colors.retain(|h| h != hex);
        self.recent_colors.insert(0, hex.to_owned());
        if self.recent_colors.len() > MAX_RECENT_COLORS {
            self.recent_colors.truncate(MAX_RECENT_COLORS);
        }
    }

    /// Records the open state of one sidebar collapsing header (CR-011 AC3).
    pub fn set_sidebar_expanded(&mut self, key: &str, open: bool) {
        self.sidebar_expanded.insert(key.to_owned(), open);
    }
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

    /// Web (S-019): loads the options from `localStorage` — the web stand-in for the
    /// config file (same JSON, same lenient loading). Never fatal: no window, no
    /// storage access, or corrupt text all yield the defaults.
    #[cfg(target_arch = "wasm32")]
    #[must_use]
    pub fn load_web() -> Self {
        web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(WEB_STORAGE_KEY).ok().flatten())
            .map(|text| serde_json::from_str(&text).unwrap_or_default())
            .unwrap_or_default()
    }

    /// Web (S-019): writes the options to `localStorage`, best-effort — the caller
    /// reports failures through the status bar, like native saves do.
    ///
    /// # Errors
    /// A message when storage is unavailable or the write fails.
    #[cfg(target_arch = "wasm32")]
    pub fn save_web(&self) -> Result<(), String> {
        let storage = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .ok_or("no access to localStorage")?;
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        storage
            .set_item(WEB_STORAGE_KEY, &json)
            .map_err(|e| format!("localStorage write failed: {e:?}"))
    }
}

/// Web (S-019): the `localStorage` key for the persisted options.
#[cfg(target_arch = "wasm32")]
const WEB_STORAGE_KEY: &str = "ascii-diagram-drawer.config";

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
            AppConfig {
                theme,
                ..Default::default()
            }
            .save(&path)
            .unwrap();
            assert_eq!(AppConfig::load(Some(&path)).theme, theme);
        }
    }

    /// FR15 (CR-009) AC6 — the recent strip deduplicates (re-recording moves the
    /// color to the front) and caps at [`MAX_RECENT_COLORS`].
    #[test]
    fn record_recent_color_deduplicates_and_caps() {
        let mut config = AppConfig::default();
        for i in 0..(MAX_RECENT_COLORS + 4) {
            config.record_recent_color(&format!("#{i:06x}"));
        }
        assert_eq!(config.recent_colors.len(), MAX_RECENT_COLORS, "capped");
        // 20 distinct colors were recorded; the strip keeps the newest 16:
        // #000004 … #000013, newest first.
        assert_eq!(config.recent_colors[0], "#000013", "newest first");
        assert!(
            !config.recent_colors.contains(&"#000003".to_owned()),
            "the oldest entries fell off"
        );

        // Re-recording an existing color moves it to the front without duplicating.
        config.record_recent_color("#00000a");
        assert_eq!(config.recent_colors[0], "#00000a");
        assert_eq!(config.recent_colors.len(), MAX_RECENT_COLORS);
        let count = config
            .recent_colors
            .iter()
            .filter(|h| *h == "#00000a")
            .count();
        assert_eq!(count, 1, "no duplicate");
    }

    /// FR6 (CR-011) AC3 — sidebar collapse states round-trip through the config
    /// file, a missing key means expanded, and a pre-feature file (no field at all)
    /// loads with everything expanded.
    #[test]
    fn sidebar_expanded_round_trip_and_absent_field_loads_expanded() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut config = AppConfig::default();
        config.set_sidebar_expanded("Built-in", false);
        config.set_sidebar_expanded("col-a/Other", true);
        config.save(&path).unwrap();

        let loaded = AppConfig::load(Some(&path));
        // Missing key = expanded, the first-use default.
        let is_expanded = |config: &AppConfig, key: &str| {
            config.sidebar_expanded.get(key).copied().unwrap_or(true)
        };
        assert!(!is_expanded(&loaded, "Built-in"), "collapsed recorded");
        assert!(
            is_expanded(&loaded, "col-a/Other"),
            "explicit open recorded"
        );
        assert!(is_expanded(&loaded, "never-seen"), "missing key = expanded");

        // A file written before CR-011 has no `sidebar_expanded` key at all.
        std::fs::write(&path, "{\"theme\": \"dark\"}\n").unwrap();
        let legacy = AppConfig::load(Some(&path));
        assert!(
            is_expanded(&legacy, "Built-in"),
            "pre-feature file → expanded"
        );
    }

    /// FR15 (CR-009) AC6 — `recent_colors` round-trips through the config file and is
    /// absent-tolerant: a pre-FR15 file (no field) loads as an empty strip.
    #[test]
    fn recent_colors_round_trip_and_absent_field_loads_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut config = AppConfig::default();
        config.record_recent_color("#123456");
        config.save(&path).unwrap();
        assert_eq!(
            AppConfig::load(Some(&path)).recent_colors,
            vec!["#123456".to_owned()]
        );

        // A file written before FR15 has no `recent_colors` key at all.
        std::fs::write(&path, "{\"theme\": \"dark\"}\n").unwrap();
        let loaded = AppConfig::load(Some(&path));
        assert!(
            loaded.recent_colors.is_empty(),
            "absent field → empty strip"
        );
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
            ..Default::default()
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
