use crate::services::settings::SettingsService;
use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Deserialize, Debug, Default)]
struct MatugenColorValue {
    color: String,
}

#[derive(Deserialize, Debug, Default)]
struct MatugenColorEntry {
    default: Option<MatugenColorValue>,
    dark: Option<MatugenColorValue>,
    light: Option<MatugenColorValue>,
}

impl MatugenColorEntry {
    fn resolve(&self, is_dark: bool) -> String {
        if is_dark {
            self.dark
                .as_ref()
                .or(self.default.as_ref())
                .or(self.light.as_ref())
                .map(|c| c.color.clone())
                .unwrap_or_else(|| "#b4c5ff".into())
        } else {
            self.light
                .as_ref()
                .or(self.default.as_ref())
                .or(self.dark.as_ref())
                .map(|c| c.color.clone())
                .unwrap_or_else(|| "#1a1b38".into())
        }
    }
}

#[derive(Deserialize, Debug)]
struct MatugenColors {
    primary: MatugenColorEntry,
    on_primary: MatugenColorEntry,
    primary_container: MatugenColorEntry,
    on_primary_container: MatugenColorEntry,
    surface: MatugenColorEntry,
    surface_container: MatugenColorEntry,
    outline: MatugenColorEntry,
    on_surface: MatugenColorEntry,
    on_surface_variant: MatugenColorEntry,
}

#[derive(Deserialize, Debug)]
struct MatugenOutput {
    colors: MatugenColors,
}

pub struct ThemeService;

impl ThemeService {
    fn get_colors_css_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".config/cos-niri/colors.css")
    }

    pub fn initialize() {
        let config_dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/cos-niri");
        let _ = std::fs::create_dir_all(config_dir);

        if !Self::get_colors_css_path().exists() {
            Self::write_fallback_theme();
        }

        Self::regenerate();
    }

    pub fn regenerate() {
        std::thread::spawn(|| {
            Self::generate_theme_from_wallpaper();
        });
    }

    fn generate_theme_from_wallpaper() {
        let path_str = std::env::var("WALLPAPER").unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".config/background")
                .to_string_lossy()
                .to_string()
        });

        if !Path::new(&path_str).exists() {
            eprintln!("[theme] Wallpaper path not found: {}", path_str);
            return;
        }

        let theme_settings = SettingsService::get_theme();
        let scheme_type = &theme_settings.scheme_type;
        let mode_str = if theme_settings.dark_mode { "dark" } else { "light" };

        eprintln!(
            "[theme] Running matugen (scheme: {}, mode: {}) on: {}",
            scheme_type, mode_str, path_str
        );

        // Always pass --source-color-index 0 and scheme type to prevent interactive prompt hangs
        let output = match Command::new("matugen")
            .args([
                "--source-color-index", "0",
                "-t", scheme_type,
                "-m", mode_str,
                "-j", "hex",
                "image", &path_str,
            ])
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[theme] Failed to execute matugen CLI: {}", e);
                return;
            }
        };

        if !output.status.success() {
            eprintln!(
                "[theme] Matugen exited with error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let matugen_data: MatugenOutput = match serde_json::from_str(&json_str) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("[theme] Failed to parse matugen JSON: {}", e);
                return;
            }
        };

        let colors = &matugen_data.colors;
        let is_dark = theme_settings.dark_mode;
        let opacity_pct = theme_settings.opacity.clamp(10, 100);
        let alpha = opacity_pct as f32 / 100.0;

        let primary = colors.primary.resolve(is_dark);
        let on_primary = colors.on_primary.resolve(is_dark);
        let primary_container_hex = colors.primary_container.resolve(is_dark);
        let primary_container = hex_to_rgba(&primary_container_hex, 0.14);
        let on_primary_container = colors.on_primary_container.resolve(is_dark);

        let surface_hex = colors.surface.resolve(is_dark);
        let surface = hex_to_rgba(&surface_hex, alpha);
        let surface_opaque = hex_to_rgba(&surface_hex, (alpha * 1.15).min(1.0));

        let surface_container_hex = colors.surface_container.resolve(is_dark);
        let surface_variant = hex_to_rgba(&surface_container_hex, (alpha * 0.12).max(0.04));

        let outline_hex = colors.outline.resolve(is_dark);
        let outline = hex_to_rgba(&outline_hex, (alpha * 0.15).max(0.06));

        let text_primary = colors.on_surface.resolve(is_dark);
        let text_secondary = colors.on_surface_variant.resolve(is_dark);
        let text_muted = outline_hex;

        let card_bg = if is_dark { "rgba(255, 255, 255, 0.05)" } else { "rgba(0, 0, 0, 0.05)" };
        let card_bg_hover = if is_dark { "rgba(255, 255, 255, 0.10)" } else { "rgba(0, 0, 0, 0.09)" };
        let bubble_bg = if is_dark { "rgba(255, 255, 255, 0.10)" } else { "rgba(0, 0, 0, 0.07)" };
        let bubble_bg_hover = if is_dark { "rgba(255, 255, 255, 0.18)" } else { "rgba(0, 0, 0, 0.12)" };
        let trough_bg = if is_dark { "rgba(255, 255, 255, 0.12)" } else { "rgba(0, 0, 0, 0.10)" };
        let sep_color = if is_dark { "rgba(255, 255, 255, 0.08)" } else { "rgba(0, 0, 0, 0.08)" };
        let sidebar_bg = if is_dark { "rgba(255, 255, 255, 0.03)" } else { "rgba(0, 0, 0, 0.03)" };

        let css = format!(
            "@define-color primary {};\n\
             @define-color on-primary {};\n\
             @define-color primary-container {};\n\
             @define-color on-primary-container {};\n\
             @define-color surface {};\n\
             @define-color surface-opaque {};\n\
             @define-color surface-variant {};\n\
             @define-color outline {};\n\
             @define-color text-primary {};\n\
             @define-color text-secondary {};\n\
             @define-color text-muted {};\n\
             @define-color card-bg {};\n\
             @define-color card-bg-hover {};\n\
             @define-color bubble-bg {};\n\
             @define-color bubble-bg-hover {};\n\
             @define-color trough-bg {};\n\
             @define-color sep-color {};\n\
             @define-color sidebar-bg {};\n",
            primary,
            on_primary,
            primary_container,
            on_primary_container,
            surface,
            surface_opaque,
            surface_variant,
            outline,
            text_primary,
            text_secondary,
            text_muted,
            card_bg,
            card_bg_hover,
            bubble_bg,
            bubble_bg_hover,
            trough_bg,
            sep_color,
            sidebar_bg
        );

        if let Ok(mut file) = File::create(Self::get_colors_css_path()) {
            let _ = file.write_all(css.as_bytes());
        }

        // Generate Niri KDL color file
        let niri_kdl = format!(
            "layout {{\n\
             \tfocus-ring {{\n\
             \t\tactive-color \"{}\"\n\
             \t}}\n\
             \tborder {{\n\
             \t\tactive-color \"{}\"\n\
             \t}}\n\
             }}",
            primary, primary
        );
        let niri_kdl_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/cos-niri/colors-niri.kdl");
        if let Ok(mut file) = File::create(niri_kdl_path) {
            let _ = file.write_all(niri_kdl.as_bytes());
        }

        // Generate Fuzzel color file
        let s_hex = strip_hex(&surface_hex);
        let p_hex = strip_hex(&primary);
        let txt_hex = strip_hex(&text_primary);
        let on_p_hex = strip_hex(&on_primary);

        let fuzzel_ini = format!(
            "[colors]\n\
             background={}fa\n\
             text={}ff\n\
             match={}ff\n\
             selection={}ff\n\
             selection-text={}ff\n\
             selection-match={}ff\n\
             border={}ff\n",
            s_hex, txt_hex, p_hex, p_hex, on_p_hex, on_p_hex, p_hex
        );
        let fuzzel_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/cos-niri/fuzzel-colors.ini");
        if let Ok(mut file) = File::create(fuzzel_path) {
            let _ = file.write_all(fuzzel_ini.as_bytes());
        }

        eprintln!("[theme] Theme generated successfully: primary={}", primary);

        // Reload Niri and notify GTK to hot-reload CSS
        let _ = Command::new("niri")
            .args(["msg", "action", "load-config-file"])
            .output();

        unsafe {
            libc::raise(libc::SIGUSR2);
        }
    }

    fn write_fallback_theme() {
        let css = "@define-color primary #b4c5ff;\n\
                   @define-color on-primary #1a1b38;\n\
                   @define-color primary-container rgba(180, 197, 255, 0.14);\n\
                   @define-color on-primary-container #d0bcff;\n\
                   @define-color surface rgba(18, 19, 26, 0.72);\n\
                   @define-color surface-variant rgba(255, 255, 255, 0.07);\n\
                   @define-color outline rgba(255, 255, 255, 0.10);\n\
                   @define-color text-primary #ffffff;\n\
                   @define-color text-secondary #c4c6d0;\n\
                   @define-color text-muted #938f99;\n";
        if let Ok(mut file) = File::create(Self::get_colors_css_path()) {
            let _ = file.write_all(css.as_bytes());
        }
    }
}

fn hex_to_rgba(hex: &str, alpha: f32) -> String {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return format!("rgba({}, {}, {}, {})", r, g, b, alpha);
        }
    }
    format!("rgba(18, 19, 26, {})", alpha)
}

fn strip_hex(hex: &str) -> &str {
    hex.trim_start_matches('#')
}
