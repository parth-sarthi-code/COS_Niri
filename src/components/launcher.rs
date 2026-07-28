use crate::components::center::CenterSection;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, FlowBox, Image, Label, Orientation,
    ScrolledWindow, SearchEntry,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

pub struct LauncherPopup {
    pub window: ApplicationWindow,
    flowbox: FlowBox,
    search_entry: SearchEntry,
}

impl LauncherPopup {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::new(app);

        // Configure Layer-Shell popup window as fullscreen
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_namespace("cos-launcher");

        // Fullscreen overlay anchors
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_anchor(Edge::Top, true);

        window.add_css_class("launcher-popup-window");

        let container = GtkBox::new(Orientation::Vertical, 24);
        container.add_css_class("launcher-popup-container");
        container.set_hexpand(true);
        container.set_vexpand(true);
        container.set_halign(gtk4::Align::Fill);
        container.set_valign(gtk4::Align::Fill);

        // Center search pill at the top
        let search_box = GtkBox::new(Orientation::Horizontal, 0);
        search_box.set_halign(gtk4::Align::Center);
        search_box.add_css_class("launcher-search-box-wrapper");

        let search_entry = SearchEntry::new();
        search_entry.add_css_class("launcher-search-pill");
        search_entry.set_placeholder_text(Some("Search Applications"));
        search_entry.set_size_request(320, -1);
        search_box.append(&search_entry);
        container.append(&search_box);

        // Grid of Apps in a ScrolledWindow (Launchpad Layout)
        let grid_scroll = ScrolledWindow::new();
        grid_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        grid_scroll.set_vexpand(true);
        grid_scroll.add_css_class("launcher-grid-scroll");

        let flowbox = FlowBox::new();
        flowbox.set_max_children_per_line(8);
        flowbox.set_min_children_per_line(6);
        flowbox.set_homogeneous(true);
        flowbox.set_row_spacing(32);
        flowbox.set_column_spacing(24);
        flowbox.set_selection_mode(gtk4::SelectionMode::None);
        flowbox.set_halign(gtk4::Align::Center);
        flowbox.add_css_class("launcher-flowbox");

        grid_scroll.set_child(Some(&flowbox));
        container.append(&grid_scroll);

        window.set_child(Some(&container));

        let popup = Self {
            window,
            flowbox,
            search_entry,
        };

        popup.refresh_apps();
        popup.setup_search_handlers();

        popup
    }

    fn refresh_apps(&self) {
        // Clear existing children
        while let Some(child) = self.flowbox.first_child() {
            self.flowbox.remove(&child);
        }

        let search_text = self.search_entry.text().to_string().to_lowercase();
        let desktop_entries = CenterSection::scan_desktop_entries();

        let mut filtered_entries = Vec::new();

        for entry in desktop_entries.values() {
            // 1. Filter out helper settings panels (e.g. bluetooth, display panels)
            let is_setting_panel = entry.categories.iter().any(|c| c == "Settings" || c == "X-GNOME-Settings-Panel")
                && entry.desktop_id != "gnome-control-center.desktop"
                && entry.desktop_id != "niri-settings.desktop"
                && !entry.name.to_lowercase().contains("system settings");

            if is_setting_panel {
                continue;
            }

            // 2. Filter by search input
            if !search_text.is_empty() {
                let name_matches = entry.name.to_lowercase().contains(&search_text);
                let exec_matches = entry.exec.to_lowercase().contains(&search_text);
                if !name_matches && !exec_matches {
                    continue;
                }
            }

            filtered_entries.push(entry.clone());
        }

        // Sort alphabetically
        filtered_entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        let self_win = self.window.clone();

        for entry in filtered_entries {
            let item_btn = Button::new();
            item_btn.add_css_class("launcher-app-item");

            let item_box = GtkBox::new(Orientation::Vertical, 8);
            item_box.set_valign(gtk4::Align::Center);
            item_box.set_halign(gtk4::Align::Center);

            let bubble = GtkBox::new(Orientation::Horizontal, 0);
            bubble.add_css_class("icon-bubble");
            bubble.set_size_request(84, 84);
            bubble.set_halign(gtk4::Align::Center);
            bubble.set_valign(gtk4::Align::Center);

            let icon_str = &entry.icon;
            let icon = if icon_str.starts_with('/') && std::path::Path::new(icon_str).exists() {
                Image::from_file(icon_str)
            } else {
                Image::from_icon_name(icon_str)
            };
            icon.set_pixel_size(56);
            icon.set_halign(gtk4::Align::Center);
            icon.set_valign(gtk4::Align::Center);
            icon.set_hexpand(true);
            icon.set_vexpand(true);
            bubble.append(&icon);
            item_box.append(&bubble);

            let name_lbl = Label::new(Some(&entry.name));
            name_lbl.add_css_class("launcher-app-name");
            name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            name_lbl.set_max_width_chars(14);
            name_lbl.set_halign(gtk4::Align::Center);
            item_box.append(&name_lbl);

            item_btn.set_child(Some(&item_box));

            let exec_cmd = entry.exec.clone();
            let s_win = self_win.clone();

            item_btn.connect_clicked(move |_| {
                CenterSection::launch_app(&exec_cmd);
                s_win.set_visible(false);
            });

            self.flowbox.append(&item_btn);
        }
    }

    fn setup_search_handlers(&self) {
        let s_ref = self.clone_ref();
        self.search_entry.connect_search_changed(move |_| {
            s_ref.refresh_apps();
        });
    }

    fn clone_ref(&self) -> Self {
        Self {
            window: self.window.clone(),
            flowbox: self.flowbox.clone(),
            search_entry: self.search_entry.clone(),
        }
    }

    pub fn toggle(&self) {
        if self.window.is_visible() {
            crate::services::animation::slide_down_close(&self.window, 56);
        } else {
            self.search_entry.set_text("");
            self.refresh_apps();
            crate::services::animation::slide_up_open(&self.window, 56);
        }
    }
}
