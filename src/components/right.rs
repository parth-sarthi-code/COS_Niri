use chrono::{Local, Timelike};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation};
use std::time::Duration;

#[allow(dead_code)]
pub struct RightSection {
    pub container: GtkBox,
    pub clock_label: Label,
    pub date_label: Label,
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

        // Wifi icon (Material Symbols: wifi U+E63E)
        let wifi_icon = Label::new(Some("\u{e63e}"));
        wifi_icon.add_css_class("ms-icon");
        wifi_icon.add_css_class("ms-icon-sm");
        wifi_icon.add_css_class("qs-icon");
        wifi_icon.set_valign(gtk4::Align::Center);
        pill.append(&wifi_icon);

        // Battery icon (Material Symbols: battery_full U+E1A5)
        let batt_icon = Label::new(Some("\u{e1a5}"));
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

        // Minute-aligned clock timer
        let clock_clone = clock_label.clone();
        let date_clone = date_label.clone();
        let secs_remaining = 60 - (Local::now().second() as u64);
        glib::timeout_add_local_once(Duration::from_secs(secs_remaining), move || {
            let now = Local::now();
            clock_clone.set_text(&now.format("%H:%M").to_string());
            date_clone.set_text(&now.format("%b %-d").to_string());

            let cc = clock_clone.clone();
            let dc = date_clone.clone();
            glib::timeout_add_seconds_local(60, move || {
                let now = Local::now();
                cc.set_text(&now.format("%H:%M").to_string());
                dc.set_text(&now.format("%b %-d").to_string());
                glib::ControlFlow::Continue
            });
        });

        Self {
            container,
            clock_label,
            date_label,
        }
    }
}
