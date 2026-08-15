use crate::services::battery::BatteryService;
use crate::services::bluetooth::BluetoothService;
use crate::services::network::NetworkService;
use crate::services::tray::TrayService;
use chrono::Local;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, ScrolledWindow};
use gtk4::gdk;
use gtk4::gdk_pixbuf::{Pixbuf, Colorspace};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    static TRAY_TEXTURE_CACHE: RefCell<HashMap<u64, gdk::Texture>> = RefCell::new(HashMap::new());
}

pub struct TrayPopup {
    pub window: gtk4::Window,
    pub container: GtkBox,
}

impl TrayPopup {
    pub fn new() -> Self {
        let window = gtk4::Window::new();
        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_namespace("cos-tray-menu");

        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_margin(Edge::Bottom, 20);
        window.set_margin(Edge::Left, 24);

        window.add_css_class("tray-popup-window");

        let container = GtkBox::new(Orientation::Vertical, 2);
        container.add_css_class("tray-popup-container");
        container.set_size_request(240, -1);

        let scroll = ScrolledWindow::new();
        scroll.add_css_class("tray-popup-scroll");
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scroll.set_max_content_height(480);
        scroll.set_propagate_natural_height(true);
        scroll.set_propagate_natural_width(true);
        scroll.set_child(Some(&container));

        window.set_child(Some(&scroll));

        Self { window, container }
    }
}

#[allow(dead_code)]
pub struct RightSection {
    pub container: GtkBox,
    pub clock_label: Label,
    pub date_label: Label,
    pub wifi_icon: Label,
    pub batt_icon: Label,
    pub bt_icon: Label,
    pub tray_popup: Rc<RefCell<Option<Rc<TrayPopup>>>>,
    pub active_id: Rc<RefCell<Option<String>>>,
}

impl RightSection {
    pub fn new<FQS, FCal, FCS, FVC>(on_toggle_qs: FQS, on_toggle_cal: FCal, on_show_click_catcher: FCS, on_visible_changed: FVC) -> Self
    where
        FQS: Fn() + 'static,
        FCal: Fn() + 'static,
        FCS: Fn() + 'static,
        FVC: Fn() + 'static,
    {
        let container = GtkBox::new(Orientation::Horizontal, 6);
        container.add_css_class("right-section");
        container.set_valign(gtk4::Align::Center);

        // System Tray Container
        let tray_box = GtkBox::new(Orientation::Horizontal, 6);
        tray_box.add_css_class("tray-container");
        tray_box.set_valign(gtk4::Align::Center);
        container.append(&tray_box);

        let tray_box_clone = tray_box.clone();
        let tray_popup = Rc::new(RefCell::new(None::<Rc<TrayPopup>>));
        let tray_popup_c = Rc::clone(&tray_popup);
        let active_id = Rc::new(RefCell::new(None::<String>));
        let active_id_c = Rc::clone(&active_id);

        let on_show_cc = Rc::new(on_show_click_catcher);
        let on_visible_changed = Rc::new(on_visible_changed);

        TrayService::global().connect_change(move || {
            let tray_box_c = tray_box_clone.clone();
            let pop_cell = Rc::clone(&tray_popup_c);
            let active_id_cell = Rc::clone(&active_id_c);
            let on_show_click_catcher_cb = Rc::clone(&on_show_cc);
            let on_vis_changed = Rc::clone(&on_visible_changed);

            glib::idle_add_local(move || {
                while let Some(child) = tray_box_c.first_child() {
                    let mut popover_child = child.first_child();
                    while let Some(c) = popover_child {
                        popover_child = c.next_sibling();
                        if c.type_().name().contains("Popover") {
                            c.unparent();
                        }
                    }
                    tray_box_c.remove(&child);
                }

                let items = TrayService::global().get_items();
                for item in &items {
                    if item.status == "Passive" {
                        continue;
                    }

                    let btn = Button::new();
                    btn.add_css_class("tray-item-btn");
                    btn.set_tooltip_text(Some(&item.title));
                    btn.set_valign(gtk4::Align::Center);
                    btn.set_size_request(32, 32);

                    let mut icon_widget = None::<gtk4::Widget>;

                    if let Some(ref pixmap) = item.pixmap {
                        if let Some(texture) = Self::texture_from_pixmap(pixmap) {
                            let img = gtk4::Image::from_paintable(Some(&texture));
                            img.set_pixel_size(15);
                            img.set_halign(gtk4::Align::Center);
                            img.set_valign(gtk4::Align::Center);
                            icon_widget = Some(img.upcast::<gtk4::Widget>());
                        }
                    }

                    if icon_widget.is_none() {
                        if let Some(ref icon_name) = item.icon_name {
                            let img = if icon_name.starts_with('/') {
                                gtk4::Image::from_file(icon_name)
                            } else {
                                gtk4::Image::from_icon_name(icon_name)
                            };
                            img.set_pixel_size(15);
                            img.set_halign(gtk4::Align::Center);
                            img.set_valign(gtk4::Align::Center);
                            icon_widget = Some(img.upcast::<gtk4::Widget>());
                        }
                    }

                    if icon_widget.is_none() {
                        let label = Label::new(Some("\u{e5c3}"));
                        label.add_css_class("ms-icon");
                        label.add_css_class("tray-fallback-icon");
                        label.set_halign(gtk4::Align::Center);
                        label.set_valign(gtk4::Align::Center);
                        icon_widget = Some(label.upcast::<gtk4::Widget>());
                    }

                    if let Some(widget) = icon_widget {
                        btn.set_child(Some(&widget));
                    }

                    let id_left = item.identifier.clone();
                    let has_menu = item.menu_path.is_some();
                    let pop_left = Rc::clone(&pop_cell);
                    let active_id_left = Rc::clone(&active_id_cell);
                    let cc_left = Rc::clone(&on_show_click_catcher_cb);
                    let btn_left = btn.clone();
                    let vis_changed_left = Rc::clone(&on_vis_changed);

                    btn.connect_clicked(move |_| {
                        if has_menu {
                            let pop_inner = {
                                let mut p_borrow = pop_left.borrow_mut();
                                if p_borrow.is_none() {
                                    let new_pop = Rc::new(TrayPopup::new());
                                    let vis_changed = Rc::clone(&vis_changed_left);
                                    new_pop.window.connect_visible_notify(move |_| {
                                        vis_changed();
                                    });
                                    *p_borrow = Some(new_pop);
                                }
                                p_borrow.as_ref().unwrap().clone()
                            };

                            let is_open = pop_inner.window.is_visible();
                            let is_same = active_id_left.borrow().as_ref() == Some(&id_left);
                            if is_open && is_same {
                                pop_inner.window.set_visible(false);
                                *active_id_left.borrow_mut() = None;
                                return;
                            }

                            let cc_inner = Rc::clone(&cc_left);
                            let id = id_left.clone();
                            let id_click = id_left.clone();
                            let id_for_get = id.clone();
                            let btn_c = btn_left.clone();
                            let active_id_inner = Rc::clone(&active_id_left);

                            TrayService::global().get_menu(&id_for_get, move |entries| {
                                if entries.is_empty() {
                                    return;
                                }

                                while let Some(child) = pop_inner.container.first_child() {
                                    pop_inner.container.remove(&child);
                                }

                                Self::append_menu_entries_win(&pop_inner.container, entries, &id, &pop_inner, 0);

                                let active_id_inner_2 = Rc::clone(&active_id_inner);
                                let id_click_2 = id_click.clone();
                                glib::idle_add_local(move || {
                                    let display = gdk::Display::default().unwrap();
                                    let monitor = display.monitors().item(0).unwrap().downcast::<gdk::Monitor>().unwrap();
                                    let monitor_width = monitor.geometry().width() as f64;

                                    let root = btn_c.root();
                                    let (x, _) = if let Some(ref r) = root {
                                        btn_c.translate_coordinates(r, 0.0, 0.0).unwrap_or((0.0, 0.0))
                                    } else {
                                        (0.0, 0.0)
                                    };
                                    let margin = (x as i32).min((monitor_width - 200.0) as i32);
                                    let margin = margin.max(24);

                                    pop_inner.window.set_margin(Edge::Left, margin);
                                    cc_inner();
                                    pop_inner.window.set_visible(true);
                                    pop_inner.window.present();
                                    *active_id_inner_2.borrow_mut() = Some(id_click_2.clone());
                                    glib::ControlFlow::Break
                                });
                            });
                        } else {
                            TrayService::global().activate(&id_left, -1, -1);
                        }
                    });

                    let id_right = item.identifier.clone();
                    let pop_right = Rc::clone(&pop_cell);
                    let active_id_right = Rc::clone(&active_id_cell);
                    let cc_right = Rc::clone(&on_show_click_catcher_cb);
                    let btn_right = btn.clone();
                    let vis_changed_right = Rc::clone(&on_vis_changed);

                    let gesture = gtk4::GestureClick::new();
                    gesture.set_button(gdk::BUTTON_SECONDARY);
                    gesture.connect_pressed(move |g, _, _, _| {
                        g.set_state(gtk4::EventSequenceState::Claimed);
                        let pop_inner = {
                            let mut p_borrow = pop_right.borrow_mut();
                            if p_borrow.is_none() {
                                let new_pop = Rc::new(TrayPopup::new());
                                let vis_changed = Rc::clone(&vis_changed_right);
                                new_pop.window.connect_visible_notify(move |_| {
                                    vis_changed();
                                });
                                *p_borrow = Some(new_pop);
                            }
                            p_borrow.as_ref().unwrap().clone()
                        };

                        let is_open = pop_inner.window.is_visible();
                        let is_same = active_id_right.borrow().as_ref() == Some(&id_right);
                        if is_open && is_same {
                            pop_inner.window.set_visible(false);
                            *active_id_right.borrow_mut() = None;
                            return;
                        }

                        let cc_inner = Rc::clone(&cc_right);
                        let id = id_right.clone();
                        let id_click = id_right.clone();
                        let id_for_get = id.clone();
                        let btn_c = btn_right.clone();
                        let active_id_inner = Rc::clone(&active_id_right);

                        TrayService::global().get_menu(&id_for_get, move |entries| {
                            if entries.is_empty() {
                                return;
                            }

                            while let Some(child) = pop_inner.container.first_child() {
                                pop_inner.container.remove(&child);
                            }

                            Self::append_menu_entries_win(&pop_inner.container, entries, &id, &pop_inner, 0);

                            let active_id_inner_2 = Rc::clone(&active_id_inner);
                            let id_click_2 = id_click.clone();
                            glib::idle_add_local(move || {
                                let display = gdk::Display::default().unwrap();
                                let monitor = display.monitors().item(0).unwrap().downcast::<gdk::Monitor>().unwrap();
                                let monitor_width = monitor.geometry().width() as f64;

                                let root = btn_c.root();
                                let (x, _) = if let Some(ref r) = root {
                                    btn_c.translate_coordinates(r, 0.0, 0.0).unwrap_or((0.0, 0.0))
                                } else {
                                    (0.0, 0.0)
                                };
                                let margin = (x as i32).min((monitor_width - 200.0) as i32);
                                let margin = margin.max(24);

                                pop_inner.window.set_margin(Edge::Left, margin);
                                cc_inner();
                                pop_inner.window.set_visible(true);
                                pop_inner.window.present();
                                 *active_id_inner_2.borrow_mut() = Some(id_click_2.clone());
                                 glib::ControlFlow::Break
                            });
                        });
                    });
                    btn.add_controller(gesture);

                    tray_box_c.append(&btn);
                }

                let active_items_count = items.iter().filter(|item| item.status != "Passive").count();
                tray_box_c.set_visible(active_items_count > 0);

                glib::ControlFlow::Break
            });
        });

        let tray_popup = tray_popup.clone();


        // 2. ChromeOS Connected Split-Pill Container (2px gap)
        let pill_group = GtkBox::new(Orientation::Horizontal, 2);
        pill_group.add_css_class("shelf-pill-group");
        pill_group.set_valign(gtk4::Align::Center);

        // --- Left Segment: Date Pill ---
        let date_btn = Button::new();
        date_btn.add_css_class("date-pill");
        date_btn.set_tooltip_text(Some("Calendar & Notifications"));
        date_btn.set_valign(gtk4::Align::Center);

        let date_label = Label::new(Some(&Local::now().format("%b %-d").to_string()));
        date_label.add_css_class("date-label");
        date_label.set_valign(gtk4::Align::Center);
        date_btn.set_child(Some(&date_label));

        date_btn.connect_clicked(move |_| {
            on_toggle_cal();
        });
        pill_group.append(&date_btn);

        // --- Right Segment: Quick Settings Pill ---
        let pill_btn = Button::new();
        pill_btn.add_css_class("qs-pill");
        pill_btn.set_tooltip_text(Some("Quick Settings"));
        pill_btn.set_valign(gtk4::Align::Center);

        let pill = GtkBox::new(Orientation::Horizontal, 6);
        pill.set_valign(gtk4::Align::Center);
        pill.set_halign(gtk4::Align::Center);
        pill.add_css_class("qs-pill-inner");

        // Clock
        let clock_label = Label::new(Some(&Local::now().format("%H:%M").to_string()));
        clock_label.add_css_class("clock-label");
        clock_label.set_valign(gtk4::Align::Center);
        pill.append(&clock_label);

        // Dropdown Arrow (▼)
        let arrow_icon = Label::new(Some("\u{e5c5}")); // arrow_drop_down
        arrow_icon.add_css_class("ms-icon");
        arrow_icon.add_css_class("ms-icon-sm");
        arrow_icon.add_css_class("qs-icon");
        arrow_icon.set_valign(gtk4::Align::Center);
        pill.append(&arrow_icon);

        // Dynamic Bluetooth icon (only visible when a device is connected)
        let bt_icon = Label::new(None);
        bt_icon.add_css_class("ms-icon");
        bt_icon.add_css_class("ms-icon-sm");
        bt_icon.add_css_class("qs-icon");
        bt_icon.set_valign(gtk4::Align::Center);
        // Initialize BT icon state
        Self::update_bt_icon(&bt_icon);
        pill.append(&bt_icon);

        // Dynamic Wifi icon
        let wifi_icon = Label::new(Some(Self::get_wifi_icon_code()));
        wifi_icon.add_css_class("ms-icon");
        wifi_icon.add_css_class("ms-icon-sm");
        wifi_icon.add_css_class("qs-icon");
        wifi_icon.set_valign(gtk4::Align::Center);
        pill.append(&wifi_icon);

        // Dynamic Battery icon
        let batt_icon = Label::new(Some(Self::get_battery_icon_code()));
        batt_icon.add_css_class("ms-icon");
        batt_icon.add_css_class("ms-icon-sm");
        batt_icon.add_css_class("qs-icon");
        batt_icon.set_valign(gtk4::Align::Center);
        pill.append(&batt_icon);

        pill_btn.set_child(Some(&pill));

        pill_btn.connect_clicked(move |_| {
            on_toggle_qs();
        });

        pill_group.append(&pill_btn);
        container.append(&pill_group);

        // Initial clock, date, battery, and bluetooth update
        let now = Local::now();
        clock_label.set_text(&now.format("%H:%M").to_string());
        date_label.set_text(&now.format("%b %-d").to_string());
        batt_icon.set_text(Self::get_battery_icon_code());

        // 1. Clock & Date: Align to next minute boundary, then 60s recurring
        let clock_c = clock_label.clone();
        let date_c = date_label.clone();
        let secs_to_next_min = (60 - chrono::Timelike::second(&now)).max(1);

        glib::timeout_add_local_once(std::time::Duration::from_secs(secs_to_next_min as u64), move || {
            let n = Local::now();
            let clock_str = n.format("%H:%M").to_string();
            let date_str = n.format("%b %-d").to_string();
            if clock_c.text() != clock_str {
                clock_c.set_text(&clock_str);
            }
            if date_c.text() != date_str {
                date_c.set_text(&date_str);
            }

            // Single recurring 60-second timer (no stacking)
            let c_c = clock_c.clone();
            let d_c = date_c.clone();
            glib::timeout_add_seconds_local(60, move || {
                let inner_n = Local::now();
                let c_str = inner_n.format("%H:%M").to_string();
                let d_str = inner_n.format("%b %-d").to_string();
                if c_c.text() != c_str {
                    c_c.set_text(&c_str);
                }
                if d_c.text() != d_str {
                    d_c.set_text(&d_str);
                }
                glib::ControlFlow::Continue
            });
        });

        // 2. Battery: Lightweight 30-second sysfs check (0 process forks)
        let batt_c = batt_icon.clone();
        glib::timeout_add_seconds_local(30, move || {
            let code = Self::get_battery_icon_code();
            if batt_c.text() != code {
                batt_c.set_text(code);
            }
            glib::ControlFlow::Continue
        });

        Self {
            container,
            clock_label,
            date_label,
            wifi_icon,
            batt_icon,
            bt_icon,
            tray_popup,
            active_id,
        }
    }

    pub fn close(&self) {
        if let Some(ref pop) = *self.tray_popup.borrow() {
            pop.window.set_visible(false);
        }
        *self.active_id.borrow_mut() = None;
    }

    pub fn is_menu_visible(&self) -> bool {
        self.tray_popup.borrow().as_ref().map(|pop| pop.window.is_visible()).unwrap_or(false)
    }

    /// Refresh main bar shelf Wi-Fi icon dynamically on D-Bus event
    pub fn update_network_state(&self) {
        let code = Self::get_wifi_icon_code();
        if self.wifi_icon.text() != code {
            self.wifi_icon.set_text(code);
        }
    }

    /// Refresh main bar shelf Bluetooth icon dynamically on bluetoothctl event
    pub fn update_bluetooth_state(&self) {
        Self::update_bt_icon(&self.bt_icon);
    }

    /// Update Bluetooth icon visibility and glyph based on connection state
    fn update_bt_icon(label: &Label) {
        if !BluetoothService::is_bluetooth_enabled() {
            if label.is_visible() {
                label.set_visible(false);
            }
            return;
        }

        let devices = BluetoothService::get_devices();
        let any_connected = devices.iter().any(|d| d.is_connected);

        if any_connected {
            let icon = "\u{e1aa}"; // bluetooth_connected (U+E1AA)
            if label.text() != icon {
                label.set_text(icon);
            }
            if !label.is_visible() {
                label.set_visible(true);
            }
        } else {
            // BT is on but nothing connected — hide from the pill to save space
            if label.is_visible() {
                label.set_visible(false);
            }
        }
    }

    /// Dynamic Wi-Fi icon — exact codepoints from Google Fonts Material Symbols
    /// ebe4 (1 bar), ebd6 (2 bar), ebe1 (3 bar), e1d8 (full)
    pub fn get_wifi_icon_code() -> &'static str {
        if !NetworkService::is_wifi_enabled() {
            return "\u{e648}"; // wifi_off
        }

        if let Some(signal) = NetworkService::get_active_signal() {
            return match signal {
                75..=100 => "\u{e1d8}",  // wifi full
                50..=74  => "\u{ebe1}",  // wifi 3 bar
                25..=49  => "\u{ebd6}",  // wifi 2 bar
                _        => "\u{ebe4}",  // wifi 1 bar
            };
        }

        // Connected but signal unavailable → full
        if NetworkService::get_active_ssid().is_some() {
            "\u{e1d8}"
        } else {
            "\u{e648}"
        }
    }

    /// Dynamic Battery icon — exact codepoints from Google Fonts Material Symbols
    /// ebdc (0 bar), ebd9 (1 bar), ebe0 (2 bar), ebdd (3 bar),
    /// ebe2 (4 bar), ebd4 (5 bar), ebd2 (6 bar)
    /// e19c (battery_alert), e2ae (charger)
    pub fn get_battery_icon_code() -> &'static str {
        let info = BatteryService::get_info();

        if !info.is_present {
            return "\u{ebd2}"; // battery_6_bar for desktop/AC
        }

        if info.status.eq_ignore_ascii_case("Charging") {
            return "\u{e2ae}"; // charger
        }

        match info.capacity {
            0..=5    => "\u{e19c}",  // battery_alert — critical
            6..=14   => "\u{ebdc}",  // battery_0_bar
            15..=28  => "\u{ebd9}",  // battery_1_bar
            29..=42  => "\u{ebe0}",  // battery_2_bar
            43..=56  => "\u{ebdd}",  // battery_3_bar
            57..=70  => "\u{ebe2}",  // battery_4_bar
            71..=84  => "\u{ebd4}",  // battery_5_bar
            85..=99  => "\u{ebd2}",  // battery_6_bar
            _        => "\u{ebd2}",  // battery_6_bar (full)
        }
    }

    fn texture_from_pixmap(pixmap: &crate::services::tray::TrayPixmap) -> Option<gdk::Texture> {
        let width = pixmap.width;
        let height = pixmap.height;
        if width <= 0 || height <= 0 {
            return None;
        }

        let raw = pixmap.buffer.as_ref();
        let len = raw.len();
        if len == 0 {
            return None;
        }

        // Fast hash of pixmap dimensions and sample bytes
        let sample1 = raw[0] as u64;
        let sample2 = raw[len / 2] as u64;
        let sample3 = raw[len - 1] as u64;
        let hash_key = (width as u64)
            ^ ((height as u64) << 16)
            ^ ((len as u64) << 32)
            ^ (sample1 << 8)
            ^ (sample2 << 24)
            ^ (sample3 << 40);

        let cached = TRAY_TEXTURE_CACHE.with(|cache| {
            cache.borrow().get(&hash_key).cloned()
        });

        if let Some(texture) = cached {
            return Some(texture);
        }

        let mut rgba = Vec::with_capacity(len);
        let mut idx = 0;
        while idx + 3 < len {
            let a = raw[idx];
            let r = raw[idx + 1];
            let g = raw[idx + 2];
            let b = raw[idx + 3];
            rgba.push(r);
            rgba.push(g);
            rgba.push(b);
            rgba.push(a);
            idx += 4;
        }

        let stride = width * 4;
        let gbytes = glib::Bytes::from_owned(rgba);
        let pixbuf = Pixbuf::from_bytes(&gbytes, Colorspace::Rgb, true, 8, width, height, stride);
        let texture = gdk::Texture::for_pixbuf(&pixbuf);

        TRAY_TEXTURE_CACHE.with(|cache| {
            let mut c = cache.borrow_mut();
            if c.len() > 50 {
                c.clear();
            }
            c.insert(hash_key, texture.clone());
        });

        Some(texture)
    }

    fn append_menu_entries_win(
        menu_box: &GtkBox,
        entries: Vec<crate::services::tray::TrayMenuEntry>,
        id: &str,
        pop: &Rc<TrayPopup>,
        indent: i32,
    ) {
        for entry in entries {
            if entry.is_separator {
                let sep = gtk4::Separator::new(Orientation::Horizontal);
                sep.add_css_class("shelf-sep");
                if indent > 0 {
                    sep.set_margin_start(indent + 12);
                }
                menu_box.append(&sep);
            } else if !entry.label.trim().is_empty() {
                let m_btn = Button::new();
                m_btn.add_css_class("qs-list-item-btn");
                m_btn.set_sensitive(entry.enabled);
                if indent > 0 {
                    m_btn.set_margin_start(indent);
                }

                let label = Label::new(Some(&entry.label));
                label.set_halign(gtk4::Align::Start);
                label.set_hexpand(true);
                label.set_max_width_chars(32);
                label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                label.set_tooltip_text(Some(&entry.label));
                m_btn.set_child(Some(&label));

                let id_click = id.to_string();
                let menu_id = entry.menu_id;
                let pop_close = Rc::clone(pop);
                let has_children = !entry.children.is_empty();

                m_btn.connect_clicked(move |_| {
                    if !has_children {
                        TrayService::global().send_menu_event(&id_click, menu_id, "clicked");
                        pop_close.window.set_visible(false);
                    }
                });

                menu_box.append(&m_btn);

                if !entry.children.is_empty() {
                    Self::append_menu_entries_win(menu_box, entry.children, id, pop, indent + 16);
                }
            }
        }
    }
}
