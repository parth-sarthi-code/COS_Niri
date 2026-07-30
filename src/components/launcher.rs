use crate::components::center::CenterSection;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, Grid, Image, Label, Orientation,
    ScrolledWindow, SearchEntry,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

struct TileWidget {
    button: Button,
    icon: Image,
    label: Label,
    exec_cmd: Rc<RefCell<String>>,
    last_icon: Rc<RefCell<String>>,
}

pub struct LauncherPopup {
    pub window: ApplicationWindow,
    search_entry: SearchEntry,
    tiles: Vec<TileWidget>,
    selected_category: Rc<RefCell<String>>,
    category_buttons: Vec<(String, Button)>,
    grid_scroll: ScrolledWindow,
}

impl LauncherPopup {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::new(app);

        // Configure Layer-Shell popup window as floating container
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_namespace("cos-launcher");

        // Centered Floating Box Layout (Tahoe Glassmorphic Launcher)
        window.set_anchor(Edge::Bottom, false);
        window.set_anchor(Edge::Top, false);
        window.set_anchor(Edge::Left, false);
        window.set_anchor(Edge::Right, false);

        window.add_css_class("tahoe-launcher-window");

        let container = GtkBox::new(Orientation::Vertical, 14);
        container.add_css_class("tahoe-launcher-container");
        container.set_size_request(620, 560);

        // --- 1. Tahoe Header (Title + Category Pills) ---
        let header_box = GtkBox::new(Orientation::Vertical, 10);
        header_box.add_css_class("tahoe-header-wrapper");

        let title_row = GtkBox::new(Orientation::Horizontal, 8);
        title_row.set_valign(gtk4::Align::Center);

        let title_icon = Label::new(Some("\u{e5c3}")); // apps icon
        title_icon.add_css_class("ms-icon");
        title_icon.add_css_class("tahoe-title-icon");
        title_row.append(&title_icon);

        let title_lbl = Label::new(Some("Applications"));
        title_lbl.add_css_class("tahoe-title-text");
        title_lbl.set_hexpand(true);
        title_lbl.set_halign(gtk4::Align::Start);
        title_row.append(&title_lbl);

        header_box.append(&title_row);

        // Category Pills Bar
        let cat_scroll = ScrolledWindow::new();
        cat_scroll.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
        cat_scroll.add_css_class("tahoe-category-scroll");

        let cat_box = GtkBox::new(Orientation::Horizontal, 6);
        cat_box.add_css_class("tahoe-category-box");

        let categories = vec![
            "All Applications",
            "Development",
            "Education",
            "Games",
            "Graphics",
            "Internet",
            "Office",
            "Settings",
            "System",
            "Utilities",
        ];

        let selected_category = Rc::new(RefCell::new("All Applications".to_string()));
        let mut category_buttons = Vec::new();

        for cat in &categories {
            let pill_btn = Button::with_label(cat);
            pill_btn.add_css_class("tahoe-category-pill");
            if *cat == "All Applications" {
                pill_btn.add_css_class("active");
            }
            cat_box.append(&pill_btn);
            category_buttons.push((cat.to_string(), pill_btn));
        }

        cat_scroll.set_child(Some(&cat_box));
        header_box.append(&cat_scroll);
        container.append(&header_box);

        // --- 2. Search Input (Interactive & Auto-Focused) ---
        let search_box = GtkBox::new(Orientation::Horizontal, 0);
        search_box.add_css_class("tahoe-search-wrapper");

        let search_entry = SearchEntry::new();
        search_entry.add_css_class("tahoe-search-pill");
        search_entry.set_placeholder_text(Some("Search applications..."));
        search_entry.set_hexpand(true);
        search_box.append(&search_entry);
        container.append(&search_box);

        // --- 3. Vertical Scrolled Window Grid (Centered Viewport) ---
        let grid_scroll = ScrolledWindow::new();
        grid_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        grid_scroll.set_overlay_scrolling(true);
        grid_scroll.set_vexpand(true);
        grid_scroll.set_hexpand(true);
        grid_scroll.set_halign(gtk4::Align::Fill);
        grid_scroll.add_css_class("tahoe-grid-scroll");

        let grid = Grid::new();
        grid.set_column_homogeneous(true);
        grid.set_row_homogeneous(false);
        grid.set_row_spacing(16);
        grid.set_column_spacing(12);
        grid.set_vexpand(true);
        grid.set_hexpand(true);
        grid.set_halign(gtk4::Align::Fill); // Fill to match search entry width
        grid.add_css_class("tahoe-app-grid");

        // Pre-allocate 60 tiles for vertical scrolling (reduces memory & DOM overhead)
        let max_tiles = 60;
        let mut tiles = Vec::with_capacity(max_tiles);
        let win_c_tile = window.clone();

        for i in 0..max_tiles {
            let row = (i / 5) as i32;
            let col = (i % 5) as i32;

            let btn = Button::new();
            btn.add_css_class("tahoe-app-item");
            btn.set_size_request(104, -1);
            btn.set_halign(gtk4::Align::Center);
            btn.set_valign(gtk4::Align::Center);

            let item_box = GtkBox::new(Orientation::Vertical, 6);
            item_box.set_valign(gtk4::Align::Center);
            item_box.set_halign(gtk4::Align::Center);

            let bubble = GtkBox::new(Orientation::Horizontal, 0);
            bubble.add_css_class("tahoe-icon-bubble");
            bubble.set_size_request(56, 56);
            bubble.set_halign(gtk4::Align::Center);
            bubble.set_valign(gtk4::Align::Center);

            let icon = Image::new();
            icon.set_pixel_size(40);
            icon.set_halign(gtk4::Align::Center);
            icon.set_valign(gtk4::Align::Center);
            bubble.append(&icon);
            item_box.append(&bubble);

            let label = Label::new(None);
            label.add_css_class("tahoe-app-name");
            label.set_wrap(true);
            label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            label.set_lines(2);
            label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            label.set_max_width_chars(13);
            label.set_width_chars(13);
            label.set_size_request(96, -1);
            label.set_halign(gtk4::Align::Center);
            item_box.append(&label);

            btn.set_child(Some(&item_box));

            let exec_cmd = Rc::new(RefCell::new(String::new()));
            let last_icon = Rc::new(RefCell::new(String::new()));
            let exec_clone = Rc::clone(&exec_cmd);
            let w_close = win_c_tile.clone();

            btn.connect_clicked(move |_| {
                let cmd = exec_clone.borrow().clone();
                if !cmd.is_empty() {
                    CenterSection::launch_app(&cmd);
                    w_close.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
                    w_close.set_visible(false);
                }
            });

            grid.attach(&btn, col, row, 1, 1);

            tiles.push(TileWidget {
                button: btn,
                icon,
                label,
                exec_cmd,
                last_icon,
            });
        }

        grid_scroll.set_child(Some(&grid));

        // Create overlay container for scrolled window to apply bottom fade gradient
        let overlay_container = gtk4::Overlay::new();
        overlay_container.set_child(Some(&grid_scroll));

        // Fade overlay box (completely click-through)
        let fade_box = GtkBox::new(Orientation::Vertical, 0);
        fade_box.add_css_class("tahoe-grid-fade");
        fade_box.set_valign(gtk4::Align::End);
        fade_box.set_height_request(40);
        fade_box.set_can_target(false);
        overlay_container.add_overlay(&fade_box);

        container.append(&overlay_container);
        window.set_child(Some(&container));

        let popup = Self {
            window,
            search_entry,
            tiles,
            selected_category,
            category_buttons,
            grid_scroll,
        };

        popup.setup_handlers();
        popup
    }

    fn setup_handlers(&self) {
        let win_focus = self.window.clone();
        let focus_controller = gtk4::EventControllerFocus::new();
        focus_controller.connect_enter(move |_| {
            win_focus.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
        });
        self.search_entry.add_controller(focus_controller);

        // Wire search input changes
        let s_self = self.clone_ref();
        self.search_entry.connect_search_changed(move |_| {
            s_self.refresh_grid();
        });

        // Wire category pill clicks
        for (cat_name, btn) in &self.category_buttons {
            let cat_str = cat_name.clone();
            let c_self = self.clone_ref();

            btn.connect_clicked(move |_| {
                *c_self.selected_category.borrow_mut() = cat_str.clone();

                // Update active pill UI class
                for (name, p_btn) in &c_self.category_buttons {
                    if *name == cat_str {
                        p_btn.add_css_class("active");
                    } else {
                        p_btn.remove_css_class("active");
                    }
                }

                c_self.refresh_grid();
            });
        }
    }

    fn clone_ref(&self) -> Self {
        Self {
            window: self.window.clone(),
            search_entry: self.search_entry.clone(),
            tiles: self.tiles.iter().map(|t| TileWidget {
                button: t.button.clone(),
                icon: t.icon.clone(),
                label: t.label.clone(),
                exec_cmd: Rc::clone(&t.exec_cmd),
                last_icon: Rc::clone(&t.last_icon),
            }).collect(),
            selected_category: Rc::clone(&self.selected_category),
            category_buttons: self.category_buttons.clone(),
            grid_scroll: self.grid_scroll.clone(),
        }
    }

    fn refresh_grid(&self) {
        let desktop_entries = CenterSection::scan_desktop_entries();
        let sel_cat = self.selected_category.borrow().clone();
        let query = self.search_entry.text().to_string().to_lowercase();

        let mut filtered = Vec::new();

        for entry in desktop_entries.values() {
            // Filter setting helper subpanels
            let is_setting_panel = entry.categories.iter().any(|c| c == "Settings" || c == "X-GNOME-Settings-Panel")
                && entry.desktop_id != "gnome-control-center.desktop"
                && entry.desktop_id != "niri-settings.desktop"
                && !entry.name.to_lowercase().contains("system settings");

            if is_setting_panel {
                continue;
            }

            // Category filter logic
            if sel_cat != "All Applications" {
                let cat_match = entry.categories.iter().any(|c| {
                    c.eq_ignore_ascii_case(&sel_cat)
                        || (sel_cat == "Development" && (c == "Utility" || c == "Development"))
                        || (sel_cat == "Internet" && (c == "Network" || c == "WebBrowser"))
                        || (sel_cat == "Utilities" && c == "Utility")
                });
                if !cat_match {
                    continue;
                }
            }

            // Search query filter logic
            if !query.is_empty() {
                let search_key = format!("{} {}", entry.name.to_lowercase(), entry.exec.to_lowercase());
                if !search_key.contains(&query) {
                    continue;
                }
            }

            filtered.push(entry);
        }

        // Sort alphabetically
        filtered.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        // Reset scroll position to top when filtering
        let vadjust = self.grid_scroll.vadjustment();
        vadjust.set_value(0.0);

        // Update pre-allocated tiles vertically
        for (i, tile) in self.tiles.iter().enumerate() {
            if i < filtered.len() {
                let entry = filtered[i];
                tile.label.set_text(&entry.name);
                *tile.exec_cmd.borrow_mut() = entry.exec.clone();

                let icon_str = &entry.icon;
                if *tile.last_icon.borrow() != *icon_str {
                    if icon_str.starts_with('/') && std::path::Path::new(icon_str).exists() {
                        tile.icon.set_from_file(Some(icon_str));
                    } else {
                        tile.icon.set_icon_name(Some(icon_str));
                    }
                    *tile.last_icon.borrow_mut() = icon_str.clone();
                }

                tile.button.set_visible(true);
            } else {
                tile.button.set_visible(false);
            }
        }
    }

    pub fn close(&self) {
        if self.window.is_visible() {
            self.window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
            crate::services::animation::slide_down_close(&self.window, 56);
        }
    }

    pub fn toggle(&self) {
        if self.window.is_visible() {
            self.close();
        } else {
            // Enable keyboard input mode when opening
            self.window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
            self.search_entry.set_text("");
            self.refresh_grid();
            self.search_entry.grab_focus();
            crate::services::animation::slide_up_open(&self.window, 56);
        }
    }
}
