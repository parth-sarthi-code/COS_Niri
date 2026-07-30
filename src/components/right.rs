use crate::services::battery::BatteryService;
use crate::services::bluetooth::BluetoothService;
use crate::services::network::NetworkService;
use chrono::Local;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation};

#[allow(dead_code)]
pub struct RightSection {
    pub container: GtkBox,
    pub clock_label: Label,
    pub date_label: Label,
    pub wifi_icon: Label,
    pub batt_icon: Label,
    pub bt_icon: Label,
}

impl RightSection {
    pub fn new<FQS, FCal>(on_toggle_qs: FQS, on_toggle_cal: FCal) -> Self
    where
        FQS: Fn() + 'static,
        FCal: Fn() + 'static,
    {
        let container = GtkBox::new(Orientation::Horizontal, 6);
        container.add_css_class("right-section");
        container.set_valign(gtk4::Align::Center);

        // 1. Stylus Button
        let stylus_btn = Button::new();
        stylus_btn.add_css_class("icon-btn-circle");
        stylus_btn.set_tooltip_text(Some("Stylus Tools"));
        stylus_btn.set_valign(gtk4::Align::Center);

        let stylus_wrap = GtkBox::new(Orientation::Horizontal, 0);
        stylus_wrap.add_css_class("icon-bubble");
        stylus_wrap.set_halign(gtk4::Align::Center);
        stylus_wrap.set_valign(gtk4::Align::Center);
        stylus_wrap.set_size_request(36, 36);

        let stylus_icon = Label::new(Some("\u{f604}"));
        stylus_icon.add_css_class("ms-icon");
        stylus_icon.add_css_class("ms-icon-sm");
        stylus_icon.set_halign(gtk4::Align::Center);
        stylus_icon.set_valign(gtk4::Align::Center);
        stylus_icon.set_hexpand(true);
        stylus_icon.set_vexpand(true);
        stylus_wrap.append(&stylus_icon);
        stylus_btn.set_child(Some(&stylus_wrap));
        container.append(&stylus_btn);

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
        }
    }

    /// Refresh main bar shelf Wi-Fi icon dynamically on D-Bus event
    pub fn update_network_state(&self) {
        let code = Self::get_wifi_icon_code();
        if self.wifi_icon.text() != code {
            self.wifi_icon.set_text(code);
        }
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
}
