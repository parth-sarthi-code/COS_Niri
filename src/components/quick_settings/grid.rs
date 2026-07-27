use crate::services::bluetooth::BluetoothService;
use crate::services::network::NetworkService;
use crate::services::night_light::NightLightService;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Grid, Label, Orientation};
use std::process::Command;

pub struct GridSection {
    pub container: Grid,
}

impl GridSection {
    pub fn new<FNet, FBt>(open_wifi_page: FNet, open_bt_page: FBt) -> Self
    where
        FNet: Fn() + Clone + 'static,
        FBt: Fn() + Clone + 'static,
    {
        let grid = Grid::new();
        grid.set_column_spacing(16);
        grid.set_row_spacing(14);
        grid.set_column_homogeneous(true);
        grid.set_row_homogeneous(true);
        grid.add_css_class("qs-grid");

        // --- ROW 0 ---

        // 1. Wi-Fi Tile (Col 0, Row 0)
        let wifi_enabled = NetworkService::is_wifi_enabled();
        let wifi_ssid = NetworkService::get_active_ssid();
        let (wifi_label, wifi_status) = if let Some(ssid) = wifi_ssid {
            (ssid, "Connected".to_string())
        } else if wifi_enabled {
            ("Wi-Fi".to_string(), "Disconnected".to_string())
        } else {
            ("Wi-Fi".to_string(), "Off".to_string())
        };

        let wifi_tile = Self::create_feature_tile(
            "\u{e63e}", // wifi icon
            &wifi_label,
            &wifi_status,
            wifi_enabled,
            true, // Has sub-panel arrow
            move |btn, sub_lbl| {
                let curr = NetworkService::is_wifi_enabled();
                let new_state = !curr;
                NetworkService::set_wifi_enabled(new_state);
                if new_state {
                    btn.add_css_class("active");
                    if let Some(lbl) = sub_lbl {
                        lbl.set_text("Connecting...");
                    }
                } else {
                    btn.remove_css_class("active");
                    if let Some(lbl) = sub_lbl {
                        lbl.set_text("Off");
                    }
                }
            },
            open_wifi_page,
        );
        grid.attach(&wifi_tile, 0, 0, 1, 1);

        // 2. Bluetooth Tile (Col 1, Row 0)
        let bt_enabled = BluetoothService::is_bluetooth_enabled();
        let bt_status = if bt_enabled { "On" } else { "Off" };

        let bt_tile = Self::create_feature_tile(
            "\u{e1a7}", // bluetooth icon
            "Bluetooth",
            bt_status,
            bt_enabled,
            true, // Has sub-panel arrow
            move |btn, sub_lbl| {
                let curr = BluetoothService::is_bluetooth_enabled();
                let new_state = !curr;
                BluetoothService::set_bluetooth_enabled(new_state);
                if new_state {
                    btn.add_css_class("active");
                    if let Some(lbl) = sub_lbl {
                        lbl.set_text("On");
                    }
                } else {
                    btn.remove_css_class("active");
                    if let Some(lbl) = sub_lbl {
                        lbl.set_text("Off");
                    }
                }
            },
            open_bt_page,
        );
        grid.attach(&bt_tile, 1, 0, 1, 1);

        // 3. Notifications / DND Tile (Col 2, Row 0)
        let dnd_tile = Self::create_feature_tile(
            "\u{e7f4}", // notifications icon
            "Notifications",
            "On, all apps",
            true,
            false,
            |_btn, _lbl| {
                let _ = Command::new("swaync-client").args(["-t", "-sw"]).spawn();
            },
            || {},
        );
        grid.attach(&dnd_tile, 2, 0, 1, 1);

        // --- ROW 1 ---

        // 4. Night Light Tile (Col 0, Row 1)
        let is_night_on = NightLightService::is_enabled();
        let night_status = if is_night_on { "On" } else { "Off" };

        let night_tile = Self::create_feature_tile(
            "\u{e51c}", // night light icon
            "Night Light",
            night_status,
            is_night_on,
            false,
            move |btn, sub_lbl| {
                let new_state = NightLightService::toggle();
                if new_state {
                    btn.add_css_class("active");
                    if let Some(lbl) = sub_lbl {
                        lbl.set_text("On");
                    }
                } else {
                    btn.remove_css_class("active");
                    if let Some(lbl) = sub_lbl {
                        lbl.set_text("Off");
                    }
                }
            },
            || {},
        );
        grid.attach(&night_tile, 0, 1, 1, 1);

        // 5. Screen Capture Tile (Col 1, Row 1)
        let capture_tile = Self::create_feature_tile(
            "\u{e412}", // camera icon
            "Screen capture",
            "",
            false,
            false,
            |_btn, _lbl| {
                let _ = Command::new("niri").args(["msg", "action", "screenshot"]).spawn();
            },
            || {},
        );
        grid.attach(&capture_tile, 1, 1, 1, 1);

        // 6. Cast Tile (Col 2, Row 1)
        let cast_tile = Self::create_feature_tile(
            "\u{e307}", // cast icon
            "Cast",
            "",
            false,
            false,
            |_btn, _lbl| {},
            || {},
        );
        grid.attach(&cast_tile, 2, 1, 1, 1);

        Self { container: grid }
    }

    fn create_feature_tile<FToggle, FArrow>(
        icon_code: &str,
        title: &str,
        sub_status: &str,
        is_active: bool,
        has_arrow: bool,
        on_toggle: FToggle,
        on_arrow: FArrow,
    ) -> GtkBox
    where
        FToggle: Fn(&Button, &Option<Label>) + 'static,
        FArrow: Fn() + 'static,
    {
        let tile_box = GtkBox::new(Orientation::Vertical, 4);
        tile_box.add_css_class("qs-tile");
        tile_box.set_halign(gtk4::Align::Center);
        tile_box.set_valign(gtk4::Align::Center);
        tile_box.set_hexpand(true);

        // Circular Icon Button
        let circle_btn = Button::new();
        circle_btn.add_css_class("qs-tile-bubble");
        circle_btn.set_size_request(52, 52);
        circle_btn.set_halign(gtk4::Align::Center);
        circle_btn.set_valign(gtk4::Align::Center);
        if is_active {
            circle_btn.add_css_class("active");
        }

        let icon = Label::new(Some(icon_code));
        icon.add_css_class("ms-icon");
        icon.set_halign(gtk4::Align::Center);
        icon.set_valign(gtk4::Align::Center);
        circle_btn.set_child(Some(&icon));

        tile_box.append(&circle_btn);

        // Text & optional sub-page arrow
        let text_box = GtkBox::new(Orientation::Horizontal, 2);
        text_box.set_halign(gtk4::Align::Center);

        let title_label = Label::new(Some(title));
        title_label.add_css_class("qs-tile-title");
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title_label.set_max_width_chars(11);
        text_box.append(&title_label);

        if has_arrow {
            let arrow_btn = Button::new();
            arrow_btn.add_css_class("qs-tile-arrow");
            let arrow_icon = Label::new(Some("\u{e5c5}")); // arrow_drop_down
            arrow_icon.add_css_class("ms-icon");
            arrow_icon.add_css_class("ms-icon-sm");
            arrow_btn.set_child(Some(&arrow_icon));
            arrow_btn.connect_clicked(move |_| {
                on_arrow();
            });
            text_box.append(&arrow_btn);
        }

        tile_box.append(&text_box);

        let sub_label = if !sub_status.is_empty() {
            let lbl = Label::new(Some(sub_status));
            lbl.add_css_class("qs-tile-sub");
            tile_box.append(&lbl);
            Some(lbl)
        } else {
            None
        };

        // Wire click listener with circle_btn and sub_label references
        let btn_clone = circle_btn.clone();
        let sub_lbl_clone = sub_label.clone();
        circle_btn.connect_clicked(move |_| {
            on_toggle(&btn_clone, &sub_lbl_clone);
        });

        tile_box
    }
}
