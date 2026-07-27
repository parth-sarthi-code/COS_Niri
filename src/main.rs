mod bar;
mod components;
mod niri_ipc;
mod services;

use bar::BarWindow;
use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{style_context_add_provider_for_display, Application, CssProvider};

const STYLE_CSS: &str = include_str!("style.css");

fn main() {
    // Fix #2: Ensure Material Symbols + Roboto fonts are installed before GTK init
    ensure_fonts_installed();

    let app = Application::builder()
        .application_id("com.chromeos.niri.bar")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_startup(|_| {
        load_css();
    });

    app.connect_activate(|app| {
        let bar = BarWindow::new(app);
        bar.show();
    });

    app.run();
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(STYLE_CSS);

    if let Some(display) = Display::default() {
        style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

/// Copy bundled fonts to ~/.local/share/fonts/cos-niri/ and run fc-cache if needed.
fn ensure_fonts_installed() {
    let font_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".local/share/fonts/cos-niri");

    // Check if Material Symbols Rounded is already installed
    let ms_installed = font_dir.join("MaterialSymbolsRounded.ttf").exists();
    let roboto_installed = font_dir.join("Roboto-Regular.ttf").exists();

    if ms_installed && roboto_installed {
        return; // Already installed
    }

    // Source fonts from the project fonts/ directory
    let project_fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");

    if !project_fonts.exists() {
        eprintln!("Warning: fonts/ directory not found at {:?}", project_fonts);
        return;
    }

    // Create target directory
    if let Err(e) = std::fs::create_dir_all(&font_dir) {
        eprintln!("Warning: failed to create font directory: {}", e);
        return;
    }

    // Copy fonts
    let fonts = [
        "MaterialSymbolsRounded.ttf",
        "Roboto-Regular.ttf",
        "Roboto-Medium.ttf",
        "Roboto-Bold.ttf",
    ];

    for font in &fonts {
        let src = project_fonts.join(font);
        let dst = font_dir.join(font);
        if src.exists() && !dst.exists() {
            if let Err(e) = std::fs::copy(&src, &dst) {
                eprintln!("Warning: failed to copy {}: {}", font, e);
            }
        }
    }

    // Run fc-cache to register fonts with fontconfig/Pango
    let _ = std::process::Command::new("fc-cache")
        .arg("-f")
        .arg(&font_dir)
        .status();
}
