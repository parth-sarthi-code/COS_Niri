use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use serde::Deserialize;

#[derive(Deserialize)]
struct MatugenColor {
    default: String,
}

#[derive(Deserialize)]
struct MatugenColors {
    primary: MatugenColor,
    on_primary: MatugenColor,
    primary_container: MatugenColor,
    on_primary_container: MatugenColor,
    surface: MatugenColor,
    surface_container: MatugenColor,
    outline: MatugenColor,
    on_surface: MatugenColor,
    on_surface_variant: MatugenColor,
}

#[derive(Deserialize)]
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
            return;
        }

        let output = match Command::new("matugen")
            .args(["-j", "hex", "image", &path_str])
            .output()
        {
            Ok(o) => o,
            Err(_) => return,
        };

        if !output.status.success() {
            return;
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let matugen_data: MatugenOutput = match serde_json::from_str(&json_str) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("[theme] Failed to parse matugen output: {}", e);
                return;
            }
        };

        let colors = &matugen_data.colors;

        let primary = &colors.primary.default;
        let on_primary = &colors.on_primary.default;
        let primary_container = hex_to_rgba(&colors.primary_container.default, 0.14);
        let on_primary_container = &colors.on_primary_container.default;
        let surface = hex_to_rgba(&colors.surface.default, 0.72);
        let surface_variant = hex_to_rgba(&colors.surface_container.default, 0.07);
        let outline = hex_to_rgba(&colors.outline.default, 0.10);
        let text_primary = &colors.on_surface.default;
        let text_secondary = &colors.on_surface_variant.default;
        let text_muted = &colors.outline.default;

        let css = format!(
            "@define-color primary {};\n\
             @define-color on-primary {};\n\
             @define-color primary-container {};\n\
             @define-color on-primary-container {};\n\
             @define-color surface {};\n\
             @define-color surface-variant {};\n\
             @define-color outline {};\n\
             @define-color text-primary {};\n\
             @define-color text-secondary {};\n\
             @define-color text-muted {};\n",
            primary,
            on_primary,
            primary_container,
            on_primary_container,
            surface,
            surface_variant,
            outline,
            text_primary,
            text_secondary,
            text_muted
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
        let s_hex = strip_hex(&colors.surface.default);
        let p_hex = strip_hex(primary);
        let fuzzel_ini = format!(
            "[colors]\n\
             background={}e6\n\
             text=ffffffff\n\
             match={}ff\n\
             selection={}4d\n\
             selection-text=ffffffff\n\
             selection-match={}ff\n\
             border={}ff\n",
            s_hex, p_hex, p_hex, p_hex, p_hex
        );
        let fuzzel_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/cos-niri/fuzzel-colors.ini");
        if let Ok(mut file) = File::create(fuzzel_path) {
            let _ = file.write_all(fuzzel_ini.as_bytes());
        }

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

        // Generate fallback Niri KDL color file
        let niri_kdl = "layout {\n\
                        \tfocus-ring {\n\
                        \t\tactive-color \"#b4c5ff\"\n\
                        \t}\n\
                        \tborder {\n\
                        \t\tactive-color \"#b4c5ff\"\n\
                        \t}\n\
                        }";
        let niri_kdl_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/cos-niri/colors-niri.kdl");
        if let Ok(mut file) = File::create(niri_kdl_path) {
            let _ = file.write_all(niri_kdl.as_bytes());
        }

        // Generate fallback Fuzzel color file
        let fuzzel_ini = "[colors]\n\
                           background=12131ae6\n\
                           text=ffffffff\n\
                           match=b4c5ffff\n\
                           selection=b4c5ff4d\n\
                           selection-text=ffffffff\n\
                           selection-match=b4c5ffff\n\
                           border=b4c5ffff\n";
        let fuzzel_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/cos-niri/fuzzel-colors.ini");
        if let Ok(mut file) = File::create(fuzzel_path) {
            let _ = file.write_all(fuzzel_ini.as_bytes());
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
