use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use image::GenericImageView;

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
        
        if let Some(color) = Self::extract_wallpaper_color() {
            Self::generate_theme(color);
        } else {
            Self::write_fallback_theme();
        }
    }

    fn extract_wallpaper_color() -> Option<(u8, u8, u8)> {
        // Retrieve wallpaper path dynamically (env var first, then default user config directory)
        let path_str = std::env::var("WALLPAPER").unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".config/background")
                .to_string_lossy()
                .to_string()
        });

        let path = Path::new(&path_str);
        if !path.exists() {
            return None;
        }

        let file = File::open(path).ok()?;
        let reader = std::io::BufReader::new(file);
        let img = image::load(reader, image::ImageFormat::Jpeg).ok()?;
        let (width, height) = img.dimensions();

        // Sample a ~64x64 grid of pixels directly to avoid downsampling allocations
        let step_x = (width / 64).max(1);
        let step_y = (height / 64).max(1);

        #[derive(Clone, Copy, Default)]
        struct ColorBin {
            r_sum: u64,
            g_sum: u64,
            b_sum: u64,
            count: u64,
        }

        // Stack-allocated color bins (zero heap allocations, O(1) space complexity)
        let mut bins = [ColorBin::default(); 16];

        for y in (0..height).step_by(step_y as usize) {
            for x in (0..width).step_by(step_x as usize) {
                if x >= width || y >= height {
                    continue;
                }
                let pixel = img.get_pixel(x, y);
                let r = pixel[0];
                let g = pixel[1];
                let b = pixel[2];

                let (h, s, l) = rgb_to_hsl(r, g, b);

                // Filter out neutral colors (greys, pure black, pure white)
                if s > 15.0 && l > 10.0 && l < 90.0 {
                    let bin_idx = ((h / 22.5) as usize) % 16;
                    let bin = &mut bins[bin_idx];
                    bin.r_sum += r as u64;
                    bin.g_sum += g as u64;
                    bin.b_sum += b as u64;
                    bin.count += 1;
                }
            }
        }

        // Find the bin with the most pixels
        let mut best_bin = 0;
        let mut max_count = 0;
        for i in 0..16 {
            if bins[i].count > max_count {
                max_count = bins[i].count;
                best_bin = i;
            }
        }

        if max_count == 0 {
            return None;
        }

        let bin = &bins[best_bin];
        Some((
            (bin.r_sum / bin.count) as u8,
            (bin.g_sum / bin.count) as u8,
            (bin.b_sum / bin.count) as u8,
        ))
    }

    fn generate_theme(color: (u8, u8, u8)) {
        let (h, s, _l) = rgb_to_hsl(color.0, color.1, color.2);
        
        // Generate Material Design 3 HSL variations
        let primary_hsl = (h, s.max(55.0), 80.0);
        let on_primary_hsl = (h, 40.0, 12.0);
        let primary_container_hsl = (h, s.max(35.0).min(50.0), 20.0);
        let on_primary_container_hsl = (h, 45.0, 85.0);
        let surface_hsl = (h, 12.0, 8.0);
        
        let primary = hsl_to_rgb(primary_hsl.0, primary_hsl.1, primary_hsl.2);
        let on_primary = hsl_to_rgb(on_primary_hsl.0, on_primary_hsl.1, on_primary_hsl.2);
        let primary_container = hsl_to_rgb(primary_container_hsl.0, primary_container_hsl.1, primary_container_hsl.2);
        let on_primary_container = hsl_to_rgb(on_primary_container_hsl.0, on_primary_container_hsl.1, on_primary_container_hsl.2);
        let surface = hsl_to_rgb(surface_hsl.0, surface_hsl.1, surface_hsl.2);
        
        let css = format!(
            "/* Generated theme - do not modify */\n\
             @define-color primary rgb({}, {}, {});\n\
             @define-color on-primary rgb({}, {}, {});\n\
             @define-color primary-container rgba({}, {}, {}, 0.14);\n\
             @define-color on-primary-container rgb({}, {}, {});\n\
             @define-color surface rgba({}, {}, {}, 0.72);\n\
             @define-color surface-opaque rgba({}, {}, {}, 0.90);\n\
             @define-color surface-variant rgba(255, 255, 255, 0.07);\n\
             @define-color outline rgba(255, 255, 255, 0.10);\n\
             @define-color text-primary #ffffff;\n\
             @define-color text-secondary #c4c6d0;\n\
             @define-color text-muted #938f99;\n",
            primary.0, primary.1, primary.2,
            on_primary.0, on_primary.1, on_primary.2,
            primary_container.0, primary_container.1, primary_container.2,
            on_primary_container.0, on_primary_container.1, on_primary_container.2,
            surface.0, surface.1, surface.2,
            surface.0, surface.1, surface.2,
        );
        
        if let Ok(mut file) = File::create(Self::get_colors_css_path()) {
            let _ = file.write_all(css.as_bytes());
        }

        // Generate Niri KDL color file
        let niri_kdl = format!(
            "layout {{\n\
             \tfocus-ring {{\n\
             \t\tactive-color \"#{:02x}{:02x}{:02x}\"\n\
             \t}}\n\
             \tborder {{\n\
             \t\tactive-color \"#{:02x}{:02x}{:02x}\"\n\
             \t}}\n\
             }}",
            primary.0, primary.1, primary.2,
            primary.0, primary.1, primary.2
        );
        let niri_kdl_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/cos-niri/colors-niri.kdl");
        if let Ok(mut file) = File::create(niri_kdl_path) {
            let _ = file.write_all(niri_kdl.as_bytes());
        }

        // Generate Fuzzel color file
        let fuzzel_ini = format!(
            "[colors]\n\
             background={:02x}{:02x}{:02x}e6\n\
             text=ffffffff\n\
             match={:02x}{:02x}{:02x}ff\n\
             selection={:02x}{:02x}{:02x}4d\n\
             selection-text=ffffffff\n\
             selection-match={:02x}{:02x}{:02x}ff\n\
             border={:02x}{:02x}{:02x}ff\n",
            surface.0, surface.1, surface.2,
            primary.0, primary.1, primary.2,
            primary.0, primary.1, primary.2,
            primary.0, primary.1, primary.2,
            primary.0, primary.1, primary.2,
        );
        let fuzzel_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/cos-niri/fuzzel-colors.ini");
        if let Ok(mut file) = File::create(fuzzel_path) {
            let _ = file.write_all(fuzzel_ini.as_bytes());
        }
    }

    fn write_fallback_theme() {
        let css = "/* Fallback theme */\n\
                   @define-color primary #b4c5ff;\n\
                   @define-color on-primary #1a1b38;\n\
                   @define-color primary-container rgba(180, 197, 255, 0.14);\n\
                   @define-color on-primary-container #d0bcff;\n\
                   @define-color surface rgba(18, 19, 26, 0.72);\n\
                   @define-color surface-opaque rgba(18, 19, 26, 0.90);\n\
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

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let min = r.min(g).min(b);
    let max = r.max(g).max(b);
    let delta = max - min;

    let mut h = 0.0;
    let mut s = 0.0;
    let l = (max + min) / 2.0;

    if delta > 0.0 {
        s = if l < 0.5 {
            delta / (max + min)
        } else {
            delta / (2.0 - max - min)
        };

        if max == r {
            h = (g - b) / delta + (if g < b { 6.0 } else { 0.0 });
        } else if max == g {
            h = (b - r) / delta + 2.0;
        } else if max == b {
            h = (r - g) / delta + 4.0;
        }
        h /= 6.0;
    }

    (h * 360.0, s * 100.0, l * 100.0)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = h / 360.0;
    let s = s / 100.0;
    let l = l / 100.0;

    let (r, g, b) = if s == 0.0 {
        (l, l, l)
    } else {
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        (
            hue_to_rgb(p, q, h + 1.0 / 3.0),
            hue_to_rgb(p, q, h),
            hue_to_rgb(p, q, h - 1.0 / 3.0),
        )
    };

    (
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 1.0 / 2.0 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}
