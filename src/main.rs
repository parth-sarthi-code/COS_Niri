mod bar;
mod components;
mod niri_ipc;
mod services;

use bar::BarWindow;
use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{style_context_add_provider_for_display, Application, CssProvider};

const COMMON_CSS: &str = include_str!("css/common.css");
const BAR_CSS: &str = include_str!("css/bar.css");
const QUICK_SETTINGS_CSS: &str = include_str!("css/quick_settings.css");
const CALENDAR_CSS: &str = include_str!("css/calendar.css");
const LAUNCHER_CSS: &str = include_str!("css/launcher.css");
const SETTINGS_CSS: &str = include_str!("css/settings.css");

thread_local! {
    static CSS_PROVIDER: CssProvider = CssProvider::new();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut initial_settings_page: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--open-settings" {
            if i + 1 < args.len() {
                initial_settings_page = Some(args[i + 1].clone());
                i += 1;
            } else {
                initial_settings_page = Some("appearance".to_string());
            }
        }
        i += 1;
    }

    // Configure GSK_RENDERER from settings.json
    let perf = services::settings::SettingsService::get_performance();
    if !perf.renderer.is_empty() {
        std::env::set_var("GSK_RENDERER", &perf.renderer);
        eprintln!("[main] Using GSK_RENDERER={}", perf.renderer);
    }

    // Generate/initialize Material You theme colors from wallpaper using matugen
    services::theme::ThemeService::initialize();

    // Set up signal handler (SIGUSR1 - 10) for on-demand theme regeneration
    glib::unix_signal_add_local(10, move || {
        eprintln!("[theme] SIGUSR1 received, regenerating theme colors...");
        services::theme::ThemeService::regenerate();
        glib::ControlFlow::Continue
    });

    // Set up signal handler (SIGUSR2 - 12) for dynamic style hot-reloads
    glib::unix_signal_add_local(12, move || {
        eprintln!("[theme] SIGUSR2 received, hot-reloading theme CSS...");
        load_css();
        glib::ControlFlow::Continue
    });

    // Fix #2: Ensure Material Symbols + Roboto fonts are installed before GTK init
    ensure_fonts_installed();

    let app = Application::builder()
        .application_id("com.chromeos.niri.bar")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    app.connect_startup(|_| {
        load_css();
    });

    let init_page = initial_settings_page.clone();
    app.connect_activate(move |app| {
        let bar = BarWindow::new(app);
        bar.show();
        if let Some(ref page) = init_page {
            bar.settings.show_page(page);
        }
    });

    app.run_with_args(&[] as &[&str]);
}

fn load_css() {
    let mut css_content = String::new();

    // Prepend generated wallpaper colors dynamically if present, otherwise fallback
    let colors_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".config/cos-niri/colors.css");

    if let Ok(colors) = std::fs::read_to_string(colors_path) {
        css_content.push_str(&colors);
    } else {
        css_content.push_str(
            "@define-color primary #b4c5ff;\n\
             @define-color on-primary #1a1b38;\n\
             @define-color primary-container rgba(180, 197, 255, 0.14);\n\
             @define-color on-primary-container #d0bcff;\n\
             @define-color surface rgba(18, 19, 26, 0.72);\n\
             @define-color surface-variant rgba(255, 255, 255, 0.07);\n\
             @define-color outline rgba(255, 255, 255, 0.10);\n\
             @define-color text-primary #ffffff;\n\
             @define-color text-secondary #c4c6d0;\n\
             @define-color text-muted #938f99;\n"
        );
    }

    css_content.push_str(COMMON_CSS);
    css_content.push_str(BAR_CSS);
    css_content.push_str(QUICK_SETTINGS_CSS);
    css_content.push_str(CALENDAR_CSS);
    css_content.push_str(LAUNCHER_CSS);
    css_content.push_str(SETTINGS_CSS);

    CSS_PROVIDER.with(|provider| {
        provider.load_from_string(&css_content);

        if let Some(display) = Display::default() {
            static ADDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !ADDED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                style_context_add_provider_for_display(
                    &display,
                    provider,
                    gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }
        }
    });
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

    let mut copied = 0;
    for font in &fonts {
        let src = project_fonts.join(font);
        let dst = font_dir.join(font);
        if src.exists() && !dst.exists() {
            if let Ok(_) = std::fs::copy(&src, &dst) {
                copied += 1;
            }
        }
    }

    if copied > 0 {
        // Run fc-cache asynchronously in background to avoid blocking GTK startup
        let target_dir = font_dir.clone();
        std::thread::spawn(move || {
            let _ = std::process::Command::new("fc-cache")
                .arg("-f")
                .arg(&target_dir)
                .status();
        });
    }
}
