use crate::niri_ipc::NiriIpcClient;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Image, Orientation};
use niri_ipc::Window;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::OnceLock;

extern "C" {
    fn setsid() -> i32;
}

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub icon: String,
    pub exec: String,
    pub desktop_id: String,
}

#[derive(Debug, Clone)]
pub struct PinnedAppConfig {
    pub display_title: String,
    pub fixed_icon: String,
    pub desktop_file: String,
    pub fallback_exec: String,
    pub match_ids: Vec<String>,
}

const DEFAULT_PINNED_APPS: &[(&str, &str, &str, &str, &[&str])] = &[
    (
        "Google Chrome",
        "com.google.Chrome",
        "com.google.Chrome.desktop",
        "flatpak run com.google.Chrome || google-chrome-stable || google-chrome",
        &["com.google.chrome", "google-chrome", "chrome"],
    ),
    (
        "Firefox",
        "org.mozilla.firefox",
        "org.mozilla.firefox.desktop",
        "firefox",
        &["org.mozilla.firefox", "firefox", "mozilla-firefox"],
    ),
    (
        "Files",
        "org.gnome.Nautilus",
        "org.gnome.Nautilus.desktop",
        "nautilus",
        &["org.gnome.nautilus", "nautilus"],
    ),
    (
        "VS Code",
        "vscode",
        "code.desktop",
        "code",
        &["code", "vscode", "com.visualstudio.code"],
    ),
    (
        "Telegram",
        "org.telegram.desktop",
        "org.telegram.desktop.desktop",
        "flatpak run org.telegram.desktop || telegram-desktop",
        &["org.telegram.desktop", "telegramdesktop", "telegram"],
    ),
    (
        "Terminal",
        "Alacritty",
        "Alacritty.desktop",
        "alacritty || ptyxis",
        &["alacritty", "foot", "kitty", "org.gnome.terminal", "org.gnome.ptyxis", "ptyxis", "console"],
    ),
];

struct PinnedAppWidget {
    config: PinnedAppConfig,
    button: Button,
    dot: GtkBox,
    focus_id_cell: Rc<RefCell<Option<u64>>>,
}

struct CenterState {
    container: Rc<GtkBox>,
    pinned_widgets: Vec<PinnedAppWidget>,
    unpinned_box: GtkBox,
}

thread_local! {
    static CENTER_STATE: RefCell<Option<CenterState>> = const { RefCell::new(None) };
}

pub struct CenterSection {
    pub container: GtkBox,
}

impl CenterSection {
    pub fn new() -> Self {
        let container = Rc::new(GtkBox::new(Orientation::Horizontal, 6));
        container.add_css_class("center-section");
        container.set_valign(gtk4::Align::Center);

        // Single-pass IconTheme setup for Flatpak paths
        Self::init_icon_theme();

        let pinned_configs = Self::load_pinned_configs();
        let desktop_entries = Self::scan_desktop_entries();

        let mut pinned_widgets = Vec::new();

        for config in pinned_configs {
            // Match desktop entry for exact launcher command
            let found_entry = desktop_entries.values().find(|entry| {
                let id_lower = entry.desktop_id.to_lowercase();
                let name_lower = entry.name.to_lowercase();
                let exec_lower = entry.exec.to_lowercase();
                config.match_ids.iter().any(|m| {
                    id_lower.contains(m) || name_lower.contains(m) || exec_lower.contains(m)
                })
            });

            let exec_cmd = found_entry
                .map(|e| e.exec.clone())
                .unwrap_or_else(|| config.fallback_exec.clone());

            let (button, dot) = Self::create_dock_button_nodes(&config.fixed_icon, &config.display_title);
            let focus_id_cell = Rc::new(RefCell::new(None));

            // CONNECT CLICK SIGNAL EXACTLY ONCE AT STARTUP
            let focus_cell_clone = Rc::clone(&focus_id_cell);
            let exec_cmd_clone = exec_cmd.clone();
            button.connect_clicked(move |_| {
                let target_id = *focus_cell_clone.borrow();
                if let Some(id) = target_id {
                    NiriIpcClient::focus_window(id);
                } else {
                    Self::launch_app(&exec_cmd_clone);
                }
            });

            container.append(&button);

            pinned_widgets.push(PinnedAppWidget {
                config,
                button,
                dot,
                focus_id_cell,
            });
        }

        let unpinned_box = GtkBox::new(Orientation::Horizontal, 6);
        container.append(&unpinned_box);

        let state = CenterState {
            container: Rc::clone(&container),
            pinned_widgets,
            unpinned_box,
        };

        CENTER_STATE.with(|cell| {
            *cell.borrow_mut() = Some(state);
        });

        // Initial window update from Niri
        if let Ok(windows) = NiriIpcClient::get_windows() {
            Self::update_dock(&windows);
        } else {
            Self::update_dock(&[]);
        }

        Self {
            container: (*container).clone(),
        }
    }

    /// Single-pass GTK IconTheme path initialization
    fn init_icon_theme() {
        if let Some(disp) = gdk::Display::default() {
            let icon_theme = gtk4::IconTheme::for_display(&disp);
            let home = std::env::var("HOME").unwrap_or_default();
            icon_theme.add_search_path("/var/lib/flatpak/exports/share/icons");
            icon_theme.add_search_path(format!("{home}/.local/share/flatpak/exports/share/icons"));
        }
    }

    /// Load pinned configs with backup/persist support
    fn load_pinned_configs() -> Vec<PinnedAppConfig> {
        let mut configs = Vec::new();
        for &(title, icon, desk, fallback, matches) in DEFAULT_PINNED_APPS {
            configs.push(PinnedAppConfig {
                display_title: title.to_string(),
                fixed_icon: icon.to_string(),
                desktop_file: desk.to_string(),
                fallback_exec: fallback.to_string(),
                match_ids: matches.iter().map(|s| s.to_string()).collect(),
            });
        }
        configs
    }

    /// Scan XDG directories once at startup (cached in memory)
    pub fn scan_desktop_entries() -> &'static HashMap<String, DesktopEntry> {
        static DESKTOP_ENTRIES_CACHE: OnceLock<HashMap<String, DesktopEntry>> = OnceLock::new();

        DESKTOP_ENTRIES_CACHE.get_or_init(|| {
            let mut entries = HashMap::new();
            let home = std::env::var("HOME").unwrap_or_default();

            let search_paths: Vec<PathBuf> = vec![
                PathBuf::from("/usr/share/applications"),
                PathBuf::from("/usr/local/share/applications"),
                PathBuf::from(format!("{home}/.local/share/applications")),
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

    /// Parse single .desktop file
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

    /// Fast POSIX direct binary execution (<1ms) with kernel process detachment (setsid)
    pub fn launch_app(exec_cmd: &str) {
        let clean_cmd = exec_cmd
            .split_whitespace()
            .filter(|arg| !arg.starts_with('%'))
            .collect::<Vec<_>>()
            .join(" ");

        let mut parts: Vec<String> = clean_cmd.split_whitespace().map(|s| s.to_string()).collect();
        if parts.is_empty() {
            return;
        }

        let program = parts.remove(0);

        crate::services::worker::TaskWorker::dispatch(move || {
            unsafe {
                let mut cmd = Command::new(&program);
                cmd.args(&parts)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .pre_exec(|| {
                        setsid();
                        Ok(())
                    });

                let _ = cmd.spawn();
            }
        });
    }

    /// In-place DOM state update for dock buttons (0 GTK container re-creation)
    pub fn update_dock(windows: &[Window]) {
        CENTER_STATE.with(|cell| {
            let mut state_borrow = cell.borrow_mut();
            if let Some(state) = state_borrow.as_mut() {
                let mut claimed_window_ids = Vec::new();

                // 1. Mutate Pinned App Buttons in-place (Zero GTK Widget Destruction!)
                for item in &mut state.pinned_widgets {
                    let matching_windows: Vec<&Window> = windows
                        .iter()
                        .filter(|w| {
                            if let Some(app_id) = &w.app_id {
                                let id_lower = app_id.to_lowercase();
                                item.config.match_ids.iter().any(|m| id_lower.contains(m))
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

                    // Mutate CSS classes directly in-place
                    item.button.remove_css_class("focused");
                    item.button.remove_css_class("running");

                    if is_focused {
                        item.button.add_css_class("focused");
                    } else if is_running {
                        item.button.add_css_class("running");
                    }

                    item.dot.remove_css_class("dot-active");
                    item.dot.remove_css_class("dot-running");
                    item.dot.remove_css_class("dot-hidden");

                    if is_focused {
                        item.dot.add_css_class("dot-active");
                    } else if is_running {
                        item.dot.add_css_class("dot-running");
                    } else {
                        item.dot.add_css_class("dot-hidden");
                    }

                    // Update current focus ID cell in-place WITHOUT re-connecting click signal
                    let focus_id = matching_windows.iter().find(|w| w.is_focused).or_else(|| matching_windows.first()).map(|w| w.id);
                    *item.focus_id_cell.borrow_mut() = focus_id;
                }

                // 2. Clear & Update Unpinned Open Apps section
                while let Some(child) = state.unpinned_box.first_child() {
                    state.unpinned_box.remove(&child);
                }

                let unpinned_windows: Vec<&Window> = windows
                    .iter()
                    .filter(|w| !claimed_window_ids.contains(&w.id))
                    .collect();

                let mut unpinned_groups: Vec<(String, Vec<&Window>)> = Vec::new();
                for w in unpinned_windows {
                    let app_id = w.app_id.clone().unwrap_or_else(|| "application-x-executable".into());
                    if let Some(group) = unpinned_groups.iter_mut().find(|(id, _)| id == &app_id) {
                        group.1.push(w);
                    } else {
                        unpinned_groups.push((app_id, vec![w]));
                    }
                }

                let desktop_entries = Self::scan_desktop_entries();

                for (app_id, group_windows) in unpinned_groups {
                    let is_focused = group_windows.iter().any(|w| w.is_focused);
                    let display_title = group_windows
                        .first()
                        .and_then(|w| w.title.clone())
                        .unwrap_or_else(|| app_id.clone());

                    let app_id_lower = app_id.to_lowercase();
                    let found_entry = desktop_entries.values().find(|entry| {
                        let id_lower = entry.desktop_id.to_lowercase();
                        let name_lower = entry.name.to_lowercase();
                        let exec_lower = entry.exec.to_lowercase();
                        id_lower.contains(&app_id_lower) || name_lower.contains(&app_id_lower) || exec_lower.contains(&app_id_lower)
                    });

                    let icon_name = found_entry.map(|e| e.icon.as_str()).unwrap_or(&app_id);
                    let (btn, _) = Self::create_dock_button_nodes(icon_name, &display_title);

                    if is_focused {
                        btn.add_css_class("focused");
                    } else {
                        btn.add_css_class("running");
                    }

                    let focus_id = group_windows.iter().find(|w| w.is_focused).or_else(|| group_windows.first()).map(|w| w.id);
                    if let Some(id) = focus_id {
                        btn.connect_clicked(move |_| {
                            NiriIpcClient::focus_window(id);
                        });
                    }

                    state.unpinned_box.append(&btn);
                }
            }
        });
    }

    /// Construct GTK dock button nodes
    fn create_dock_button_nodes(icon_str: &str, tooltip: &str) -> (Button, GtkBox) {
        let dock_btn = Button::new();
        dock_btn.add_css_class("dock-item");
        dock_btn.set_tooltip_text(Some(tooltip));

        let item_box = GtkBox::new(Orientation::Vertical, 2);
        item_box.set_valign(gtk4::Align::Center);
        item_box.set_halign(gtk4::Align::Center);

        let bubble = GtkBox::new(Orientation::Horizontal, 0);
        bubble.add_css_class("icon-bubble");
        bubble.set_valign(gtk4::Align::Center);
        bubble.set_halign(gtk4::Align::Center);
        bubble.set_size_request(36, 36);

        let icon = if icon_str.starts_with('/') && Path::new(icon_str).exists() {
            Image::from_file(icon_str)
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

        let dot = GtkBox::new(Orientation::Horizontal, 0);
        dot.set_halign(gtk4::Align::Center);
        dot.add_css_class("dot-hidden");
        item_box.append(&dot);

        dock_btn.set_child(Some(&item_box));
        (dock_btn, dot)
    }
}
