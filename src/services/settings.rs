use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Pinned app entry persisted in settings.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedApp {
    pub desktop_id: String,
    pub name: String,
}

/// Per-module blur/xray configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlurSettings {
    pub bar_blur: bool,
    pub quick_settings_blur: bool,
    pub calendar_blur: bool,
    pub launcher_blur: bool,
    #[serde(default = "default_true")]
    pub tray_blur: bool,
    #[serde(default = "default_true")]
    pub fuzzel_blur: bool,
    pub bar_xray: bool,
    pub quick_settings_xray: bool,
    pub calendar_xray: bool,
    pub launcher_xray: bool,
    #[serde(default)]
    pub tray_xray: bool,
    #[serde(default)]
    pub fuzzel_xray: bool,
}

fn default_true() -> bool {
    true
}

impl Default for BlurSettings {
    fn default() -> Self {
        Self {
            bar_blur: true,
            quick_settings_blur: true,
            calendar_blur: true,
            launcher_blur: true,
            tray_blur: true,
            fuzzel_blur: true,
            bar_xray: true,
            quick_settings_xray: false,
            calendar_xray: false,
            launcher_xray: false,
            tray_xray: false,
            fuzzel_xray: false,
        }
    }
}

/// Material You Theme Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSettings {
    pub scheme_type: String,
    pub dark_mode: bool,
    #[serde(default = "default_opacity")]
    pub opacity: u32,
}

fn default_opacity() -> u32 {
    78
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            scheme_type: "scheme-tonal-spot".into(),
            dark_mode: true,
            opacity: 78,
        }
    }
}

/// Performance & Rendering Engine Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSettings {
    #[serde(default = "default_renderer")]
    pub renderer: String, // "vulkan", "gl", "cairo"
    #[serde(default = "default_launcher_backend")]
    pub launcher_backend: String, // "builtin", "fuzzel"
}

fn default_renderer() -> String {
    "vulkan".into()
}

fn default_launcher_backend() -> String {
    "builtin".into()
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            renderer: "vulkan".into(),
            launcher_backend: "builtin".into(),
        }
    }
}

/// Root settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub pinned_apps: Vec<PinnedApp>,
    pub blur: BlurSettings,
    pub wallpaper_path: String,
    #[serde(default)]
    pub theme: ThemeSettings,
    #[serde(default)]
    pub performance: PerformanceSettings,
}

impl Default for Settings {
    fn default() -> Self {
        // Seed pinned apps from the hardcoded defaults
        let default_pinned = vec![
            PinnedApp { desktop_id: "com.google.Chrome.desktop".into(), name: "Google Chrome".into() },
            PinnedApp { desktop_id: "org.mozilla.firefox.desktop".into(), name: "Firefox".into() },
            PinnedApp { desktop_id: "org.gnome.Nautilus.desktop".into(), name: "Files".into() },
            PinnedApp { desktop_id: "code.desktop".into(), name: "VS Code".into() },
            PinnedApp { desktop_id: "org.telegram.desktop.desktop".into(), name: "Telegram".into() },
            PinnedApp { desktop_id: "Alacritty.desktop".into(), name: "Terminal".into() },
        ];

        let wallpaper = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/background")
            .to_string_lossy()
            .to_string();

        Self {
            pinned_apps: default_pinned,
            blur: BlurSettings::default(),
            wallpaper_path: wallpaper,
            theme: ThemeSettings::default(),
            performance: PerformanceSettings::default(),
        }
    }
}

static SETTINGS: OnceLock<Mutex<Settings>> = OnceLock::new();

pub struct SettingsService;

impl SettingsService {
    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".config/cos-niri/settings.json")
    }

    /// Load settings from disk, or create defaults if none exist
    fn load_or_default() -> Settings {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<Settings>(&data) {
                    return settings;
                }
                eprintln!("[settings] Failed to parse settings.json, using defaults");
            }
        }
        let defaults = Settings::default();
        // Write defaults to disk immediately
        Self::write_to_disk(&defaults);
        defaults
    }

    /// Write settings to disk
    fn write_to_disk(settings: &Settings) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(settings) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    eprintln!("[settings] Failed to write settings.json: {}", e);
                }
            }
            Err(e) => eprintln!("[settings] Failed to serialize settings: {}", e),
        }
    }

    /// Get singleton reference
    fn global() -> &'static Mutex<Settings> {
        SETTINGS.get_or_init(|| Mutex::new(Self::load_or_default()))
    }

    /// Get a clone of current settings
    pub fn get() -> Settings {
        Self::global().lock().unwrap().clone()
    }

    /// Get pinned apps list
    pub fn get_pinned_apps() -> Vec<PinnedApp> {
        Self::global().lock().unwrap().pinned_apps.clone()
    }

    /// Get blur settings
    pub fn get_blur() -> BlurSettings {
        Self::global().lock().unwrap().blur.clone()
    }

    /// Get wallpaper path
    pub fn get_wallpaper_path() -> String {
        Self::global().lock().unwrap().wallpaper_path.clone()
    }

    /// Add a pinned app
    pub fn pin_app(desktop_id: &str, name: &str) {
        let mut settings = Self::global().lock().unwrap();
        // Don't duplicate
        if settings.pinned_apps.iter().any(|a| a.desktop_id == desktop_id) {
            return;
        }
        settings.pinned_apps.push(PinnedApp {
            desktop_id: desktop_id.to_string(),
            name: name.to_string(),
        });
        Self::write_to_disk(&settings);
    }

    /// Remove a pinned app
    pub fn unpin_app(desktop_id: &str) {
        let mut settings = Self::global().lock().unwrap();
        settings.pinned_apps.retain(|a| a.desktop_id != desktop_id);
        Self::write_to_disk(&settings);
    }

    /// Update blur setting for a specific module
    pub fn set_blur(module: &str, enabled: bool) {
        let mut settings = Self::global().lock().unwrap();
        match module {
            "bar" => settings.blur.bar_blur = enabled,
            "quick_settings" => settings.blur.quick_settings_blur = enabled,
            "calendar" => settings.blur.calendar_blur = enabled,
            "launcher" => settings.blur.launcher_blur = enabled,
            "tray" => settings.blur.tray_blur = enabled,
            "fuzzel" => settings.blur.fuzzel_blur = enabled,
            _ => return,
        }
        Self::write_to_disk(&settings);
    }

    /// Update xray setting for a specific module
    pub fn set_xray(module: &str, enabled: bool) {
        let mut settings = Self::global().lock().unwrap();
        match module {
            "bar" => settings.blur.bar_xray = enabled,
            "quick_settings" => settings.blur.quick_settings_xray = enabled,
            "calendar" => settings.blur.calendar_xray = enabled,
            "launcher" => settings.blur.launcher_xray = enabled,
            "tray" => settings.blur.tray_xray = enabled,
            "fuzzel" => settings.blur.fuzzel_xray = enabled,
            _ => return,
        }
        Self::write_to_disk(&settings);
    }

    /// Update wallpaper path
    pub fn set_wallpaper_path(path: &str) {
        let mut settings = Self::global().lock().unwrap();
        settings.wallpaper_path = path.to_string();
        Self::write_to_disk(&settings);
    }

    /// Get current theme settings
    pub fn get_theme() -> ThemeSettings {
        Self::global().lock().unwrap().theme.clone()
    }

    /// Set Matugen color scheme type (e.g. scheme-tonal-spot, scheme-neutral, etc.)
    pub fn set_scheme_type(scheme_type: &str) {
        let mut settings = Self::global().lock().unwrap();
        settings.theme.scheme_type = scheme_type.to_string();
        Self::write_to_disk(&settings);
    }

    /// Set Matugen mode (dark/light)
    pub fn set_dark_mode(dark_mode: bool) {
        let mut settings = Self::global().lock().unwrap();
        settings.theme.dark_mode = dark_mode;
        Self::write_to_disk(&settings);
    }

    /// Set surface opacity percentage (10..100)
    pub fn set_opacity(opacity: u32) {
        let mut settings = Self::global().lock().unwrap();
        settings.theme.opacity = opacity.clamp(10, 100);
        Self::write_to_disk(&settings);
    }

    /// Get performance settings
    pub fn get_performance() -> PerformanceSettings {
        Self::global().lock().unwrap().performance.clone()
    }

    /// Set GSK Renderer engine ("vulkan", "gl", "cairo")
    pub fn set_renderer(renderer: &str) {
        let mut settings = Self::global().lock().unwrap();
        settings.performance.renderer = renderer.to_string();
        Self::write_to_disk(&settings);
    }

    /// Set App Launcher Backend ("builtin", "fuzzel")
    pub fn set_launcher_backend(backend: &str) {
        let mut settings = Self::global().lock().unwrap();
        settings.performance.launcher_backend = backend.to_string();
        Self::write_to_disk(&settings);
    }

    /// Reset theme, blur, effects, and performance back to factory defaults
    pub fn reset_to_defaults() {
        let mut settings = Self::global().lock().unwrap();
        settings.blur = BlurSettings::default();
        settings.theme = ThemeSettings::default();
        settings.performance = PerformanceSettings::default();
        Self::write_to_disk(&settings);
    }
}
