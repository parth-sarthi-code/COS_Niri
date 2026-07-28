use crate::services::battery::BatteryService;
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
}

impl RightSection {
    pub fn new<F>(on_toggle_qs: F) -> Self
    where
        F: Fn() + 'static,
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

        date_btn.connect_clicked(|_| {
            // Calendar integration slot
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

        // 5-second dynamic polling loop to update time, date, wifi, and battery
        let clock_c = clock_label.clone();
        let date_c = date_label.clone();
        let wifi_c = wifi_icon.clone();
        let batt_c = batt_icon.clone();

        glib::timeout_add_seconds_local(5, move || {
            let now = Local::now();
            clock_c.set_text(&now.format("%H:%M").to_string());
            date_c.set_text(&now.format("%b %-d").to_string());
            wifi_c.set_text(Self::get_wifi_icon_code());
            batt_c.set_text(Self::get_battery_icon_code());
            glib::ControlFlow::Continue
        });

        Self {
            container,
            clock_label,
            date_label,
            wifi_icon,
            batt_icon,
        }
    }

    /// Dynamic Wi-Fi icon code based on connection state & signal strength
    pub fn get_wifi_icon_code() -> &'static str {
        if !NetworkService::is_wifi_enabled() {
            return "\u{e648}"; // wifi_off
        }
        if NetworkService::get_active_ssid().is_some() || NetworkService::get_active_signal().is_some() {
            "\u{e1d8}" // network_wifi (codepoint: e1d8)
        } else {
            "\u{e648}" // disconnected
        }
    }

    /// Dynamic Battery icon code based on BatteryService status & percentage
    pub fn get_battery_icon_code() -> &'static str {
        let info = BatteryService::get_info();
        if !info.is_present {
            return "\u{e2ae}"; // charger icon for desktop AC power
        }

        if info.status.eq_ignore_ascii_case("Charging") {
            "\u{e2ae}" // charger icon
        } else if info.capacity < 10 {
            "\u{e19c}" // battery_alert (less than 10%)
        } else if info.capacity < 25 {
            "\u{ebd9}" // battery_1_bar
        } else if info.capacity < 40 {
            "\u{ebdc}" // battery_2_bar
        } else if info.capacity < 55 {
            "\u{ebdf}" // battery_3_bar
        } else if info.capacity < 70 {
            "\u{ebe2}" // battery_4_bar
        } else if info.capacity < 85 {
            "\u{ebe4}" // battery_5_bar
        } else if info.capacity < 95 {
            "\u{ebe5}" // battery_6_bar
        } else {
            "\u{e1a4}" // battery_full
        }
    }
}
