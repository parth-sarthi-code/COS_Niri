use crate::niri_ipc::NiriIpcClient;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Image, Orientation};
use niri_ipc::Window;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub icon: String,
    pub exec: String,
    pub desktop_id: String,
}

struct PinnedAppConfig {
    display_title: &'static str,
    fixed_icon: &'static str,
    desktop_file: &'static str,
    fallback_exec: &'static str,
    match_ids: &'static [&'static str],
}

const PINNED_APPS: &[PinnedAppConfig] = &[
    PinnedAppConfig {
        display_title: "Google Chrome",
        fixed_icon: "com.google.Chrome",
        desktop_file: "com.google.Chrome.desktop",
        fallback_exec: "flatpak run com.google.Chrome || google-chrome-stable || google-chrome",
        match_ids: &["com.google.chrome", "google-chrome", "chrome"],
    },
    PinnedAppConfig {
        display_title: "Firefox",
        fixed_icon: "org.mozilla.firefox",
        desktop_file: "org.mozilla.firefox.desktop",
        fallback_exec: "firefox",
        match_ids: &["org.mozilla.firefox", "firefox", "mozilla-firefox"],
    },
    PinnedAppConfig {
        display_title: "Files",
        fixed_icon: "org.gnome.Nautilus",
        desktop_file: "org.gnome.Nautilus.desktop",
        fallback_exec: "nautilus",
        match_ids: &["org.gnome.nautilus", "nautilus"],
    },
    PinnedAppConfig {
        display_title: "VS Code",
        fixed_icon: "vscode",
        desktop_file: "code.desktop",
        fallback_exec: "code",
        match_ids: &["code", "vscode", "com.visualstudio.code"],
    },
    PinnedAppConfig {
        display_title: "Telegram",
        fixed_icon: "org.telegram.desktop",
        desktop_file: "org.telegram.desktop.desktop",
        fallback_exec: "flatpak run org.telegram.desktop || telegram-desktop",
        match_ids: &["org.telegram.desktop", "telegramdesktop", "telegram"],
    },
    PinnedAppConfig {
        display_title: "Terminal",
        fixed_icon: "Alacritty",
        desktop_file: "Alacritty.desktop",
        fallback_exec: "alacritty || ptyxis",
        match_ids: &["alacritty", "foot", "kitty", "org.gnome.terminal", "org.gnome.ptyxis", "ptyxis", "console"],
    },
];

thread_local! {
    static CENTER_BOX: RefCell<Option<Rc<GtkBox>>> = const { RefCell::new(None) };
}

pub struct CenterSection {
    pub container: GtkBox,
}

impl CenterSection {
    pub fn new() -> Self {
        let container = Rc::new(GtkBox::new(Orientation::Horizontal, 6));
        container.add_css_class("center-section");
        container.set_valign(gtk4::Align::Center);

        CENTER_BOX.with(|cell| {
            *cell.borrow_mut() = Some(Rc::clone(&container));
        });

        // Initial update with windows from Niri
        if let Ok(windows) = NiriIpcClient::get_windows() {
            Self::update_dock(&windows);
        } else {
            Self::update_dock(&[]);
        }

        Self {
            container: (*container).clone(),
        }
    }

    /// Scan system and Flatpak XDG directories for .desktop entries once at startup (cached in memory)
    pub fn scan_desktop_entries() -> &'static HashMap<String, DesktopEntry> {
        static DESKTOP_ENTRIES_CACHE: std::sync::OnceLock<HashMap<String, DesktopEntry>> =
            std::sync::OnceLock::new();

        DESKTOP_ENTRIES_CACHE.get_or_init(|| {
            let mut entries = HashMap::new();
            let home = std::env::var("HOME").unwrap_or_default();

            let search_paths: Vec<PathBuf> = vec![
                PathBuf::from("/usr/share/applications"),
                PathBuf::from("/usr/local/share/applications"),
                PathBuf::from(format!("{home}/.local/share/applications")),
                // Flatpak export paths
                PathBuf::from("/var/lib/flatpak/exports/share/applications"),
                PathBuf::from(format!("{home}/.local/share/flatpak/exports/share/applications")),
            ];

            for path in search_paths {
                if let Ok(read_dir) = fs::read_dir(path) {
                    for entry in read_dir.flatten() {
                        let file_path = entry.path();
                        if file_path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                            if let Some(desktop_entry) = Self::parse_desktop_file(&file_path) {
                                entries.insert(desktop_entry.desktop_id.clone(), desktop_entry);
                            }
                        }
                    }
                }
            }

            entries
        })
    }

    /// Parse a single .desktop file in pure Rust
    fn parse_desktop_file(path: &Path) -> Option<DesktopEntry> {
        let content = fs::read_to_string(path).ok()?;
        let desktop_id = path.file_name()?.to_str()?.to_string();

        let mut name = None;
        let mut icon = None;
        let mut exec = None;
        let mut in_desktop_entry = false;

        for line in content.lines() {
            let line = line.trim();
            if line == "[Desktop Entry]" {
                in_desktop_entry = true;
                continue;
            } else if line.starts_with('[') && line.ends_with(']') {
                in_desktop_entry = false;
            }

            if in_desktop_entry {
                if line.starts_with("Name=") && name.is_none() {
                    name = Some(line["Name=".len()..].to_string());
                } else if line.starts_with("Icon=") && icon.is_none() {
                    icon = Some(line["Icon=".len()..].to_string());
                } else if line.starts_with("Exec=") && exec.is_none() {
                    let raw_exec = &line["Exec=".len()..];
                    let clean_exec = raw_exec
                        .split_whitespace()
                        .filter(|arg| !arg.starts_with('%'))
                        .collect::<Vec<_>>()
                        .join(" ");
                    exec = Some(clean_exec);
                }
            }
        }

        if let (Some(name), Some(icon), Some(exec)) = (name, icon, exec) {
            Some(DesktopEntry {
                name,
                icon,
                exec,
                desktop_id,
            })
        } else {
            None
        }
    }

    /// Launch an application instantly without shell overhead using TaskWorker
    pub fn launch_app(exec_cmd: &str) {
        let clean_cmd = exec_cmd
            .split_whitespace()
            .filter(|arg| !arg.starts_with('%'))
            .collect::<Vec<_>>()
            .join(" ");

        let parts: Vec<&str> = clean_cmd.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let program = parts[0].to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        crate::services::worker::TaskWorker::dispatch(move || {
            let _ = Command::new(&program)
                .args(&args)
                .spawn();
        });
    }

    /// Refresh dock icons based on live open windows from Niri and parsed Desktop entries.
    pub fn update_dock(windows: &[Window]) {
        CENTER_BOX.with(|cell| {
            if let Some(center_box) = cell.borrow().as_ref() {
                // Clear existing buttons
                while let Some(child) = center_box.first_child() {
                    center_box.remove(&child);
                }

                let desktop_entries = Self::scan_desktop_entries();

                // Track which windows are claimed by pinned apps
                let mut claimed_window_ids = Vec::new();

                // 1. Build Pinned App Buttons
                for pinned in PINNED_APPS {
                    let matching_windows: Vec<&Window> = windows
                        .iter()
                        .filter(|w| {
                            if let Some(app_id) = &w.app_id {
                                let id_lower = app_id.to_lowercase();
                                pinned.match_ids.iter().any(|m| id_lower.contains(m))
                            } else {
                                false
                            }
                        })
                        .collect();

                    for w in &matching_windows {
                        claimed_window_ids.push(w.id);
                    }

                    let is_running = !matching_windows.is_empty();
                    let is_focused = matching_windows.iter().any(|w| w.is_focused);

                    // Find desktop entry for this pinned app to get the exact launcher command
                    let found_entry = desktop_entries.values().find(|entry| {
                        let id_lower = entry.desktop_id.to_lowercase();
                        let name_lower = entry.name.to_lowercase();
                        let exec_lower = entry.exec.to_lowercase();
                        pinned.match_ids.iter().any(|m| {
                            id_lower.contains(m) || name_lower.contains(m) || exec_lower.contains(m)
                        })
                    });

                    let exec_cmd = found_entry
                        .map(|e| e.exec.clone())
                        .unwrap_or_else(|| pinned.fallback_exec.to_string());

                    // Pinned apps ALWAYS use their fixed icon so the dock icon never changes when running
                    let btn = Self::create_dock_button(
                        pinned.fixed_icon,
                        pinned.display_title,
                        is_running,
                        is_focused,
                    );

                    let focus_id = matching_windows.iter().find(|w| w.is_focused).or_else(|| matching_windows.first()).map(|w| w.id);

                    btn.connect_clicked(move |_| {
                        if let Some(id) = focus_id {
                            NiriIpcClient::focus_window(id);
                        } else {
                            Self::launch_app(&exec_cmd);
                        }
                    });

                    center_box.append(&btn);
                }

                // 2. Build Unpinned Open Apps (appearing automatically to the right)
                let unpinned_windows: Vec<&Window> = windows
                    .iter()
                    .filter(|w| !claimed_window_ids.contains(&w.id))
                    .collect();

                // Group unpinned windows by app_id
                let mut unpinned_groups: Vec<(String, Vec<&Window>)> = Vec::new();
                for w in unpinned_windows {
                    let app_id = w.app_id.clone().unwrap_or_else(|| "application-x-executable".into());
                    if let Some(group) = unpinned_groups.iter_mut().find(|(id, _)| id == &app_id) {
                        group.1.push(w);
                    } else {
                        unpinned_groups.push((app_id, vec![w]));
                    }
                }

                for (app_id, group_windows) in unpinned_groups {
                    let is_focused = group_windows.iter().any(|w| w.is_focused);
                    let display_title = group_windows
                        .first()
                        .and_then(|w| w.title.clone())
                        .unwrap_or_else(|| app_id.clone());

                    // Try to match desktop entry for unpinned app
                    let app_id_lower = app_id.to_lowercase();
                    let found_entry = desktop_entries.values().find(|entry| {
                        let id_lower = entry.desktop_id.to_lowercase();
                        let name_lower = entry.name.to_lowercase();
                        let exec_lower = entry.exec.to_lowercase();
                        id_lower.contains(&app_id_lower) || name_lower.contains(&app_id_lower) || exec_lower.contains(&app_id_lower)
                    });

                    let icon_name = found_entry.map(|e| e.icon.as_str()).unwrap_or(&app_id);

                    let btn = Self::create_dock_button(
                        icon_name,
                        &display_title,
                        true, // Always running since it's an open window
                        is_focused,
                    );

                    let focus_id = group_windows.iter().find(|w| w.is_focused).or_else(|| group_windows.first()).map(|w| w.id);
                    if let Some(id) = focus_id {
                        btn.connect_clicked(move |_| {
                            NiriIpcClient::focus_window(id);
                        });
                    }

                    center_box.append(&btn);
                }
            }
        });
    }

    /// Helper to construct GTK dock button supporting icon names or absolute file paths.
    fn create_dock_button(
        icon_str: &str,
        tooltip: &str,
        is_running: bool,
        is_focused: bool,
    ) -> Button {
        let dock_btn = Button::new();
        dock_btn.add_css_class("dock-item");
        if is_focused {
            dock_btn.add_css_class("focused");
        } else if is_running {
            dock_btn.add_css_class("running");
        }
        dock_btn.set_tooltip_text(Some(tooltip));

        let item_box = GtkBox::new(Orientation::Vertical, 2);
        item_box.set_valign(gtk4::Align::Center);
        item_box.set_halign(gtk4::Align::Center);

        // Circular icon bubble
        let bubble = GtkBox::new(Orientation::Horizontal, 0);
        bubble.add_css_class("icon-bubble");
        bubble.set_valign(gtk4::Align::Center);
        bubble.set_halign(gtk4::Align::Center);
        bubble.set_size_request(36, 36);

        // Resolve icon (supports absolute file paths or themed icon names, with Flatpak icon search paths)
        let icon = if icon_str.starts_with('/') && Path::new(icon_str).exists() {
            Image::from_file(icon_str)
        } else if let Some(disp) = gtk4::gdk::Display::default() {
            let icon_theme = gtk4::IconTheme::for_display(&disp);

            // Add Flatpak search paths to GTK IconTheme
            let home = std::env::var("HOME").unwrap_or_default();
            icon_theme.add_search_path("/var/lib/flatpak/exports/share/icons");
            icon_theme.add_search_path(format!("{home}/.local/share/flatpak/exports/share/icons"));

            if icon_theme.has_icon(icon_str) {
                Image::from_icon_name(icon_str)
            } else {
                Image::from_icon_name("application-x-executable")
            }
        } else {
            Image::from_icon_name(icon_str)
        };

        icon.set_pixel_size(24);
        icon.set_halign(gtk4::Align::Center);
        icon.set_valign(gtk4::Align::Center);
        icon.set_hexpand(true);
        icon.set_vexpand(true);
        bubble.append(&icon);
        item_box.append(&bubble);

        // Dot indicator for running/focused state
        let dot = GtkBox::new(Orientation::Horizontal, 0);
        dot.set_halign(gtk4::Align::Center);
        if is_focused {
            dot.add_css_class("dot-active");
        } else if is_running {
            dot.add_css_class("dot-running");
        } else {
            dot.add_css_class("dot-hidden");
        }
        item_box.append(&dot);

        dock_btn.set_child(Some(&item_box));
        dock_btn
    }
}
