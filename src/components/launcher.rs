use crate::components::center::CenterSection;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, FlowBox, Image, Label, Orientation,
    ScrolledWindow,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

pub struct LauncherPopup {
    pub window: ApplicationWindow,
    flowbox: FlowBox,
    current_category: Rc<RefCell<String>>,
}

const CATEGORIES: &[(&str, &[&str])] = &[
    ("All", &[]),
    ("Education", &["Education"]),
    ("Games", &["Game"]),
    ("Graphics", &["Graphics"]),
    ("Help", &["Help"]),
    ("Internet", &["Network", "WebBrowser", "Email"]),
    ("Multimedia", &["AudioVideo", "Audio", "Video", "Player"]),
    ("Office", &["Office"]),
    ("System", &["System", "Settings"]),
    ("Utilities", &["Utility", "Accessories"]),
];

impl LauncherPopup {
    pub fn new(app: &Application) -> Self {
        let window = ApplicationWindow::new(app);

        // Configure Layer-Shell popup window
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_namespace("cos-launcher");

        // Center on screen by disabling all edge anchors
        window.set_anchor(Edge::Bottom, false);
        window.set_anchor(Edge::Left, false);
        window.set_anchor(Edge::Right, false);
        window.set_anchor(Edge::Top, false);

        window.add_css_class("launcher-popup-window");

        let container = GtkBox::new(Orientation::Vertical, 16);
        container.add_css_class("launcher-popup-container");
        container.set_size_request(780, 580);

        // 1. Header (Left: Icon + "Applications", Right: Options Button)
        let header = GtkBox::new(Orientation::Horizontal, 0);
        header.add_css_class("launcher-header");

        let title_box = GtkBox::new(Orientation::Horizontal, 8);
        title_box.set_hexpand(true);
        title_box.set_halign(gtk4::Align::Start);
        title_box.set_valign(gtk4::Align::Center);

        let apps_icon = Label::new(Some("\u{e5c3}")); // apps icon
        apps_icon.add_css_class("ms-icon");
        apps_icon.add_css_class("launcher-header-icon");
        title_box.append(&apps_icon);

        let title_label = Label::new(Some("Applications"));
        title_label.add_css_class("launcher-title");
        title_box.append(&title_label);

        header.append(&title_box);

        // Options button (...) on the right
        let options_btn = Button::new();
        options_btn.add_css_class("launcher-options-btn");
        let options_icon = Label::new(Some("\u{e5d4}")); // more_vert icon
        options_icon.add_css_class("ms-icon");
        options_icon.add_css_class("ms-icon-sm");
        options_btn.set_child(Some(&options_icon));
        header.append(&options_btn);

        container.append(&header);

        // 2. Categories Horizontal Tab Bar
        let categories_scroll = ScrolledWindow::new();
        categories_scroll.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Never);
        categories_scroll.set_vexpand(false);
        categories_scroll.add_css_class("launcher-categories-scroll");

        let categories_box = GtkBox::new(Orientation::Horizontal, 8);
        categories_box.add_css_class("launcher-categories-box");

        categories_scroll.set_child(Some(&categories_box));
        container.append(&categories_scroll);

        // 3. Grid of Apps in a ScrolledWindow
        let grid_scroll = ScrolledWindow::new();
        grid_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        grid_scroll.set_vexpand(true);
        grid_scroll.add_css_class("launcher-grid-scroll");

        let flowbox = FlowBox::new();
        flowbox.set_max_children_per_line(5);
        flowbox.set_min_children_per_line(5);
        flowbox.set_homogeneous(true);
        flowbox.set_row_spacing(18);
        flowbox.set_column_spacing(18);
        flowbox.set_selection_mode(gtk4::SelectionMode::None);
        flowbox.add_css_class("launcher-flowbox");

        grid_scroll.set_child(Some(&flowbox));
        container.append(&grid_scroll);

        window.set_child(Some(&container));

        let current_category = Rc::new(RefCell::new("All".to_string()));

        let popup = Self {
            window,
            flowbox,
            current_category,
        };

        popup.init_categories(categories_box);
        popup.refresh_apps();

        popup
    }

    fn init_categories(&self, categories_box: GtkBox) {
        let current_cat = Rc::clone(&self.current_category);
        let self_ref = self.clone_ref();

        let buttons = Rc::new(RefCell::new(Vec::<Button>::new()));

        for &(cat_name, _) in CATEGORIES {
            let cat_btn = Button::with_label(cat_name);
            cat_btn.add_css_class("launcher-cat-btn");
            if cat_name == "All" {
                cat_btn.add_css_class("active");
            }

            let c_name = cat_name.to_string();
            let c_cat = Rc::clone(&current_cat);
            let s_ref = self_ref.clone_ref();
            let btns_ref = Rc::clone(&buttons);

            cat_btn.connect_clicked(move |_| {
                *c_cat.borrow_mut() = c_name.clone();

                for btn in btns_ref.borrow().iter() {
                    let label = btn.label().unwrap_or_default();
                    if label.as_str() == c_name {
                        btn.add_css_class("active");
                    } else {
                        btn.remove_css_class("active");
                    }
                }

                s_ref.refresh_apps();
            });

            categories_box.append(&cat_btn);
            buttons.borrow_mut().push(cat_btn);
        }
    }

    fn refresh_apps(&self) {
        // Clear existing children
        while let Some(child) = self.flowbox.first_child() {
            self.flowbox.remove(&child);
        }

        let cat = self.current_category.borrow().clone();
        let desktop_entries = CenterSection::scan_desktop_entries();

        let mut filtered_entries = Vec::new();

        let matched_categories = CATEGORIES
            .iter()
            .find(|(name, _)| name == &cat)
            .map(|(_, list)| *list)
            .unwrap_or(&[]);

        for entry in desktop_entries.values() {
            if cat != "All" {
                let matches_cat = entry.categories.iter().any(|c| {
                    matched_categories.iter().any(|m| c.eq_ignore_ascii_case(m))
                });
                if !matches_cat {
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

            let item_box = GtkBox::new(Orientation::Vertical, 4);
            item_box.set_valign(gtk4::Align::Center);
            item_box.set_halign(gtk4::Align::Center);

            let bubble = GtkBox::new(Orientation::Horizontal, 0);
            bubble.add_css_class("icon-bubble");
            bubble.set_size_request(56, 56);
            bubble.set_halign(gtk4::Align::Center);
            bubble.set_valign(gtk4::Align::Center);

            let icon_str = &entry.icon;
            let icon = if icon_str.starts_with('/') && std::path::Path::new(icon_str).exists() {
                Image::from_file(icon_str)
            } else {
                Image::from_icon_name(icon_str)
            };
            icon.set_pixel_size(36);
            icon.set_halign(gtk4::Align::Center);
            icon.set_valign(gtk4::Align::Center);
            icon.set_hexpand(true);
            icon.set_vexpand(true);
            bubble.append(&icon);
            item_box.append(&bubble);

            let name_lbl = Label::new(Some(&entry.name));
            name_lbl.add_css_class("launcher-app-name");
            name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            name_lbl.set_max_width_chars(11);
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

    fn clone_ref(&self) -> Self {
        Self {
            window: self.window.clone(),
            flowbox: self.flowbox.clone(),
            current_category: Rc::clone(&self.current_category),
        }
    }

    pub fn toggle(&self) {
        if self.window.is_visible() {
            crate::services::animation::slide_down_close(&self.window, 56);
        } else {
            self.refresh_apps();
            crate::services::animation::slide_up_open(&self.window, 56);
        }
    }
}
