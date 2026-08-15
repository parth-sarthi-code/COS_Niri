use crate::niri_ipc::NiriIpcClient;
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Image, Orientation};
use niri_ipc::Window;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

extern "C" {
    fn setsid() -> i32;
    fn inotify_init1(flags: i32) -> i32;
    fn inotify_add_watch(fd: i32, pathname: *const i8, mask: u32) -> i32;
    fn read(fd: i32, buf: *mut std::ffi::c_void, count: usize) -> isize;
}

const IN_CLOEXEC: i32 = 0x80000;
const IN_CREATE: u32 = 0x00000100;
const IN_DELETE: u32 = 0x00000200;
const IN_MODIFY: u32 = 0x00000002;
const IN_MOVED_TO: u32 = 0x00000080;
const IN_MOVED_FROM: u32 = 0x00000040;

/// Dirty flag for desktop entry cache — starts true to trigger initial scan
static DESKTOP_ENTRIES_DIRTY: AtomicBool = AtomicBool::new(true);
static DESKTOP_ENTRIES_CACHE: OnceLock<Mutex<Arc<HashMap<String, DesktopEntry>>>> = OnceLock::new();
static WATCHER_STARTED: AtomicBool = AtomicBool::new(false);

// Icon texture cache — avoids repeated GTK icon theme lookups for the same icon name
thread_local! {
    static ICON_CACHE: RefCell<HashMap<String, gtk4::IconPaintable>> = RefCell::new(HashMap::new());
}

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    pub name: String,
    pub icon: String,
    pub exec: String,
    pub desktop_id: String,
    pub categories: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PinnedAppConfig {
    pub display_title: String,
    pub fixed_icon: String,
    pub desktop_file: String,
    pub fallback_candidates: Vec<String>,
    pub match_ids: Vec<String>,
}

const DEFAULT_PINNED_APPS: &[(&str, &str, &str, &[&str], &[&str])] = &[
    (
        "Google Chrome",
        "com.google.Chrome",
        "com.google.Chrome.desktop",
        &["google-chrome-stable", "google-chrome", "flatpak run com.google.Chrome"],
        &["com.google.chrome", "google-chrome", "chrome"],
    ),
    (
        "Firefox",
        "org.mozilla.firefox",
        "org.mozilla.firefox.desktop",
        &["firefox", "flatpak run org.mozilla.firefox"],
        &["org.mozilla.firefox", "firefox", "mozilla-firefox"],
    ),
    (
        "Files",
        "org.gnome.Nautilus",
        "org.gnome.Nautilus.desktop",
        &["nautilus", "org.gnome.Nautilus"],
        &["org.gnome.nautilus", "nautilus"],
    ),
    (
        "VS Code",
        "vscode",
        "code.desktop",
        &["code", "codium", "flatpak run com.visualstudio.code"],
        &["code", "vscode", "com.visualstudio.code"],
    ),
    (
        "Telegram",
        "org.telegram.desktop",
        "org.telegram.desktop.desktop",
        &["telegram-desktop", "telegram", "flatpak run org.telegram.desktop"],
        &["org.telegram.desktop", "telegramdesktop", "telegram"],
    ),
    (
        "Terminal",
        "Alacritty",
        "Alacritty.desktop",
        &["alacritty", "foot", "kitty", "ptyxis"],
        &["alacritty", "foot", "kitty", "org.gnome.terminal", "org.gnome.ptyxis", "ptyxis", "console"],
    ),
];

#[allow(dead_code)]
struct PinnedAppWidget {
    config: PinnedAppConfig,
    button: Button,
    dot: GtkBox,
    focus_id_cell: Rc<RefCell<Option<u64>>>,
    resolved_exec: String,
}

struct UnpinnedAppWidget {
    button: Button,
    focus_id_cell: Rc<RefCell<Option<u64>>>,
}

#[allow(dead_code)]
struct CenterState {
    container: Rc<GtkBox>,
    pinned_widgets: Vec<PinnedAppWidget>,
    unpinned_box: GtkBox,
    unpinned_widgets: HashMap<String, UnpinnedAppWidget>,
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
        container.set_halign(gtk4::Align::Center);
        container.set_hexpand(false);

        Self::init_icon_theme();

        let desktop_entries = Self::scan_desktop_entries();
        let pinned_configs = Self::load_pinned_configs(&desktop_entries);

        let mut pinned_widgets = Vec::new();

        for config in pinned_configs {
            let widget = Self::create_pinned_widget(config, &desktop_entries);
            container.append(&widget.button);
            pinned_widgets.push(widget);
        }

        let unpinned_box = GtkBox::new(Orientation::Horizontal, 6);
        container.append(&unpinned_box);

        let state = CenterState {
            container: Rc::clone(&container),
            pinned_widgets,
            unpinned_box,
            unpinned_widgets: HashMap::new(),
        };

        CENTER_STATE.with(|cell| {
            *cell.borrow_mut() = Some(state);
        });

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

    /// Probe candidate binary commands against PATH and return the first valid executable
    fn probe_first_valid_binary(candidates: &[String]) -> String {
        let path_var = std::env::var("PATH").unwrap_or_default();
        let path_dirs: Vec<PathBuf> = std::env::split_paths(&path_var).collect();

        for candidate in candidates {
            let parts: Vec<&str> = candidate.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let bin_name = parts[0];

            // Direct executable path check
            if bin_name.starts_with('/') {
                if Path::new(bin_name).exists() {
                    return candidate.clone();
                }
                continue;
            }

            // Probe in PATH directories
            for dir in &path_dirs {
                let full_path = dir.join(bin_name);
                if full_path.is_file() {
                    return candidate.clone();
                }
            }
        }

        // Fallback to first candidate if none probed
        candidates.first().cloned().unwrap_or_default()
    }

    /// Load pinned configs — reads from settings.json first, falls back to hardcoded defaults
    fn load_pinned_configs(desktop_entries: &HashMap<String, DesktopEntry>) -> Vec<PinnedAppConfig> {
        let saved = crate::services::settings::SettingsService::get_pinned_apps();

        if saved.is_empty() {
            // Fallback to hardcoded defaults
            let mut configs = Vec::new();
            for &(title, icon, desk, fallbacks, matches) in DEFAULT_PINNED_APPS {
                configs.push(PinnedAppConfig {
                    display_title: title.to_string(),
                    fixed_icon: icon.to_string(),
                    desktop_file: desk.to_string(),
                    fallback_candidates: fallbacks.iter().map(|s| s.to_string()).collect(),
                    match_ids: matches.iter().map(|s| s.to_string()).collect(),
                });
            }
            return configs;
        }

        let mut configs = Vec::new();

        for app in &saved {
            // Try to find matching hardcoded config for enriched metadata
            let hardcoded = DEFAULT_PINNED_APPS.iter().find(|&&(_, _, desk, _, _)| desk == app.desktop_id);

            if let Some(&(title, icon, desk, fallbacks, matches)) = hardcoded {
                configs.push(PinnedAppConfig {
                    display_title: title.to_string(),
                    fixed_icon: icon.to_string(),
                    desktop_file: desk.to_string(),
                    fallback_candidates: fallbacks.iter().map(|s| s.to_string()).collect(),
                    match_ids: matches.iter().map(|s| s.to_string()).collect(),
                });
            } else {
                // Dynamically resolve from desktop entry
                let entry = desktop_entries.values().find(|e| e.desktop_id == app.desktop_id);
                let icon = entry.map(|e| e.icon.clone()).unwrap_or_else(|| "application-x-executable".to_string());
                let exec = entry.map(|e| e.exec.clone()).unwrap_or_default();
                let desktop_id_lower = app.desktop_id.to_lowercase().replace(".desktop", "");
                let name_lower = app.name.to_lowercase();
                let mut match_ids = vec![desktop_id_lower.clone(), name_lower];
                if let Some(first_exec) = exec.split_whitespace().next() {
                    let bin_clean = first_exec.split('/').last().unwrap_or(first_exec).to_lowercase();
                    if !match_ids.contains(&bin_clean) {
                        match_ids.push(bin_clean);
                    }
                }

                configs.push(PinnedAppConfig {
                    display_title: app.name.clone(),
                    fixed_icon: icon,
                    desktop_file: app.desktop_id.clone(),
                    fallback_candidates: if exec.is_empty() { vec![] } else { vec![exec] },
                    match_ids,
                });
            }
        }

        configs
    }

    /// Build a single pinned app widget
    fn create_pinned_widget(config: PinnedAppConfig, desktop_entries: &HashMap<String, DesktopEntry>) -> PinnedAppWidget {
        // 1. Try to resolve Exec= from scanned desktop entry
        let found_entry = desktop_entries.values().find(|entry| {
            let id_lower = entry.desktop_id.to_lowercase();
            let name_lower = entry.name.to_lowercase();
            let exec_lower = entry.exec.to_lowercase();
            config.match_ids.iter().any(|m| {
                id_lower.contains(m) || name_lower.contains(m) || exec_lower.contains(m)
            })
        });

        // 2. If desktop entry not found, probe candidate binaries on system PATH
        let resolved_exec = found_entry
            .map(|e| e.exec.clone())
            .unwrap_or_else(|| Self::probe_first_valid_binary(&config.fallback_candidates));

        let icon_to_use = found_entry
            .map(|e| e.icon.clone())
            .unwrap_or_else(|| config.fixed_icon.clone());

        let (button, dot) = Self::create_dock_button_nodes(&icon_to_use, &config.display_title);
        let focus_id_cell = Rc::new(RefCell::new(None));

        let focus_cell_clone = Rc::clone(&focus_id_cell);
        let exec_cmd_clone = resolved_exec.clone();

        button.connect_clicked(move |_| {
            let target_id = *focus_cell_clone.borrow();
            let mut focus_success = false;

            if let Some(id) = target_id {
                if let Ok(windows) = NiriIpcClient::get_windows() {
                    if windows.iter().any(|w| w.id == id) {
                        NiriIpcClient::focus_window(id);
                        focus_success = true;
                    }
                }
            }

            if !focus_success {
                Self::launch_app(&exec_cmd_clone);
            }
        });

        PinnedAppWidget {
            config,
            button,
            dot,
            focus_id_cell,
            resolved_exec,
        }
    }

    /// Hot-reload pinned apps from settings.json and update the live dock immediately
    pub fn reload_pinned_apps() {
        CENTER_STATE.with(|cell| {
            let mut state_borrow = cell.borrow_mut();
            if let Some(state) = state_borrow.as_mut() {
                // 1. Remove existing pinned widgets from container
                for item in &state.pinned_widgets {
                    state.container.remove(&item.button);
                }
                state.container.remove(&state.unpinned_box);

                // 2. Build new pinned widgets from settings.json
                let desktop_entries = Self::scan_desktop_entries();
                let pinned_configs = Self::load_pinned_configs(&desktop_entries);
                let mut new_pinned_widgets = Vec::new();

                for config in pinned_configs {
                    let widget = Self::create_pinned_widget(config, &desktop_entries);
                    state.container.append(&widget.button);
                    new_pinned_widgets.push(widget);
                }

                // 3. Re-append unpinned_box at the end
                state.container.append(&state.unpinned_box);
                state.pinned_widgets = new_pinned_widgets;

                // 4. Update running/focus state immediately
                if let Ok(windows) = NiriIpcClient::get_windows() {
                    drop(state_borrow);
                    Self::update_dock(&windows);
                } else {
                    drop(state_borrow);
                    Self::update_dock(&[]);
                }
            }
        });
    }

    /// Scan XDG application directories. Uses an Arc-wrapped cache with inotify-driven
    /// dirty-flag invalidation: O(0) filesystem I/O and zero lock contention or deadlock risk.
    pub fn scan_desktop_entries() -> Arc<HashMap<String, DesktopEntry>> {
        // Ensure the inotify watcher thread is started exactly once
        if !WATCHER_STARTED.swap(true, Ordering::SeqCst) {
            Self::start_desktop_watcher();
        }

        let cache = DESKTOP_ENTRIES_CACHE.get_or_init(|| Mutex::new(Arc::new(HashMap::new())));
        let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());

        if DESKTOP_ENTRIES_DIRTY.swap(false, Ordering::SeqCst) || guard.is_empty() {
            // Re-scan all XDG application directories into a new map
            let mut new_entries = HashMap::new();
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
                                new_entries.insert(desktop_entry.desktop_id.clone(), desktop_entry);
                            }
                        }
                    }
                }
            }

            // Clear icon cache when desktop entries change (icons may have changed)
            ICON_CACHE.with(|cache| cache.borrow_mut().clear());

            eprintln!("[center] Desktop entry cache refreshed: {} entries", new_entries.len());
            *guard = Arc::new(new_entries);
        }

        Arc::clone(&guard)
    }

    /// Spawn a background inotify watcher thread monitoring XDG application directories.
    /// Sets DESKTOP_ENTRIES_DIRTY=true on any file create/delete/modify/move events.
    fn start_desktop_watcher() {
        std::thread::spawn(move || unsafe {
            let fd = inotify_init1(IN_CLOEXEC);
            if fd < 0 {
                eprintln!("[center] Failed to initialize inotify for desktop watcher");
                return;
            }

            let home = std::env::var("HOME").unwrap_or_default();
            let watch_dirs = [
                "/usr/share/applications".to_string(),
                "/usr/local/share/applications".to_string(),
                format!("{home}/.local/share/applications"),
                "/var/lib/flatpak/exports/share/applications".to_string(),
                format!("{home}/.local/share/flatpak/exports/share/applications"),
            ];

            let mask = IN_CREATE | IN_DELETE | IN_MODIFY | IN_MOVED_TO | IN_MOVED_FROM;

            for dir in &watch_dirs {
                if Path::new(dir).is_dir() {
                    if let Ok(c_path) = CString::new(dir.as_bytes()) {
                        inotify_add_watch(fd, c_path.as_ptr(), mask);
                    }
                }
            }

            let mut buffer = [0u8; 4096];

            // Kernel blocks with 0.0% CPU until a .desktop file event occurs
            loop {
                let bytes_read = read(fd, buffer.as_mut_ptr() as *mut std::ffi::c_void, buffer.len());
                if bytes_read <= 0 {
                    break;
                }
                DESKTOP_ENTRIES_DIRTY.store(true, Ordering::SeqCst);
            }
        });
    }

    /// Parse single .desktop file with specifier stripping
    fn parse_desktop_file(path: &Path) -> Option<DesktopEntry> {
        let content = fs::read_to_string(path).ok()?;
        let desktop_id = path.file_name()?.to_str()?.to_string();

        let mut name = None;
        let mut icon = None;
        let mut exec = None;
        let mut categories = Vec::new();
        let mut in_desktop_entry = false;
        let mut no_display = false;

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
                } else if line.starts_with("Categories=") {
                    let cats_str = &line["Categories=".len()..];
                    categories = cats_str
                        .split(';')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                } else if line.starts_with("NoDisplay=") {
                    let val = line["NoDisplay=".len()..].trim().to_lowercase();
                    if val == "true" {
                        no_display = true;
                    }
                }
            }
        }

        if no_display {
            return None;
        }

        if let (Some(name), Some(icon), Some(exec)) = (name, icon, exec) {
            Some(DesktopEntry {
                name,
                icon,
                exec,
                desktop_id,
                categories,
            })
        } else {
            None
        }
    }

    /// Fast POSIX direct binary execution (<1ms) with kernel process detachment (setsid)
    pub fn launch_app(exec_cmd: &str) {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_double_quotes = false;
        let mut in_single_quotes = false;
        let mut escaped = false;

        for c in exec_cmd.chars() {
            if escaped {
                current.push(c);
                escaped = false;
            } else if c == '\\' && !in_single_quotes {
                escaped = true;
            } else if c == '"' && !in_single_quotes {
                in_double_quotes = !in_double_quotes;
            } else if c == '\'' && !in_double_quotes {
                in_single_quotes = !in_single_quotes;
            } else if c.is_whitespace() && !in_double_quotes && !in_single_quotes {
                if !current.is_empty() {
                    let first_char = current.chars().next();
                    if first_char != Some('%') {
                        parts.push(current.clone());
                    }
                    current.clear();
                }
            } else {
                current.push(c);
            }
        }
        if !current.is_empty() {
            let first_char = current.chars().next();
            if first_char != Some('%') {
                parts.push(current);
            }
        }

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

    /// Optimized in-place DOM state updates for both pinned and unpinned dock buttons
    pub fn update_dock(windows: &[Window]) {
        CENTER_STATE.with(|cell| {
            let mut state_borrow = cell.borrow_mut();
            if let Some(state) = state_borrow.as_mut() {
                let mut claimed_window_ids = Vec::new();

                // 1. Mutate Pinned App Buttons in-place
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

                    let focus_id = matching_windows.iter().find(|w| w.is_focused).or_else(|| matching_windows.first()).map(|w| w.id);
                    *item.focus_id_cell.borrow_mut() = focus_id;
                }

                // 2. Optimized Unpinned Open Apps with caching
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

                let current_unpinned_ids: Vec<String> = unpinned_groups.iter().map(|(id, _)| id.clone()).collect();
                let old_unpinned_ids: Vec<String> = state.unpinned_widgets.keys().cloned().collect();

                // Remove closed unpinned widgets
                for old_id in old_unpinned_ids {
                    if !current_unpinned_ids.contains(&old_id) {
                        if let Some(widget) = state.unpinned_widgets.remove(&old_id) {
                            state.unpinned_box.remove(&widget.button);
                        }
                    }
                }

                let desktop_entries = Self::scan_desktop_entries();

                for (app_id, group_windows) in unpinned_groups {
                    let is_focused = group_windows.iter().any(|w| w.is_focused);
                    let focus_id = group_windows.iter().find(|w| w.is_focused).or_else(|| group_windows.first()).map(|w| w.id);

                    if let Some(widget) = state.unpinned_widgets.get(&app_id) {
                        // Mutate in-place
                        widget.button.remove_css_class("focused");
                        widget.button.remove_css_class("running");

                        if is_focused {
                            widget.button.add_css_class("focused");
                        } else {
                            widget.button.add_css_class("running");
                        }

                        *widget.focus_id_cell.borrow_mut() = focus_id;
                    } else {
                        // Create new unpinned widget
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

                        let focus_id_cell = Rc::new(RefCell::new(focus_id));
                        let focus_cell_clone = Rc::clone(&focus_id_cell);
                        btn.connect_clicked(move |_| {
                            let target_id = *focus_cell_clone.borrow();
                            if let Some(id) = target_id {
                                if let Ok(windows) = NiriIpcClient::get_windows() {
                                    if windows.iter().any(|w| w.id == id) {
                                        NiriIpcClient::focus_window(id);
                                    }
                                }
                            }
                        });

                        state.unpinned_box.append(&btn);
                        state.unpinned_widgets.insert(
                            app_id,
                            UnpinnedAppWidget {
                                button: btn,
                                focus_id_cell,
                            },
                        );
                    }
                }
            }
        });
    }

    /// Construct GTK dock button nodes with icon texture caching
    fn create_dock_button_nodes(icon_str: &str, tooltip: &str) -> (Button, GtkBox) {
        let dock_btn = Button::new();
        dock_btn.add_css_class("dock-item");
        dock_btn.set_tooltip_text(Some(tooltip));
        dock_btn.set_halign(gtk4::Align::Center);
        dock_btn.set_hexpand(false);

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
            // Try cached icon paintable first
            let cached = ICON_CACHE.with(|cache| {
                cache.borrow().get(icon_str).cloned()
            });

            if let Some(paintable) = cached {
                Image::from_paintable(Some(&paintable))
            } else {
                // Resolve via icon theme and cache the result
                if let Some(display) = gdk::Display::default() {
                    let icon_theme = gtk4::IconTheme::for_display(&display);
                    let paintable = icon_theme.lookup_icon(
                        icon_str,
                        &[],   // no fallback names
                        24,
                        1,     // scale factor
                        gtk4::TextDirection::None,
                        gtk4::IconLookupFlags::empty(),
                    );
                    ICON_CACHE.with(|cache| {
                        cache.borrow_mut().insert(icon_str.to_string(), paintable.clone());
                    });
                    Image::from_paintable(Some(&paintable))
                } else {
                    Image::from_icon_name(icon_str)
                }
            }
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
