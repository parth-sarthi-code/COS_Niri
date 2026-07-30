use crate::services::bluetooth::BluetoothService;
use crate::services::network::NetworkService;
use crate::services::night_light::NightLightService;
use crate::services::power_profile::{PowerProfile, PowerProfileService};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Grid, Label, Orientation};
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

pub struct GridSection {
    pub container: Grid,
    pub wifi_btn: Button,
    pub wifi_title: Label,
    pub wifi_sub: Label,
    pub bt_btn: Button,
    pub bt_sub: Label,
    pub night_btn: Button,
    pub night_sub: Label,
    pub power_btn: Button,
    pub power_icon: Label,
    pub power_sub: Label,
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
        let (wifi_label, wifi_status) = if let Some(ref ssid) = wifi_ssid {
            (ssid.as_str(), "Connected")
        } else if wifi_enabled {
            ("Wi-Fi", "Disconnected")
        } else {
            ("Wi-Fi", "Off")
        };

        let (wifi_tile, wifi_btn, wifi_title, wifi_sub, _) = Self::create_feature_tile(
            "\u{e63e}", // wifi icon
            wifi_label,
            wifi_status,
            wifi_enabled,
            true, // Has sub-panel arrow
            move |btn, sub_lbl, _| {
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

        let (bt_tile, bt_btn, _bt_title, bt_sub, _) = Self::create_feature_tile(
            "\u{e1a7}", // bluetooth icon
            "Bluetooth",
            bt_status,
            bt_enabled,
            true, // Has sub-panel arrow
            move |btn, sub_lbl, _| {
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
        let (dnd_tile, _, _, _, _) = Self::create_feature_tile(
            "\u{e7f4}", // notifications icon
            "Notifications",
            "On, all apps",
            true,
            false,
            |_btn, _lbl, _| {
                let _ = Command::new("swaync-client").args(["-t", "-sw"]).spawn();
            },
            || {},
        );
        grid.attach(&dnd_tile, 2, 0, 1, 1);

        // --- ROW 1 ---

        // 4. Night Light Tile (Col 0, Row 1)
        let is_night_on = NightLightService::is_enabled();
        let night_status = if is_night_on { "On" } else { "Off" };

        let (night_tile, night_btn, _night_title, night_sub, _) = Self::create_feature_tile(
            "\u{e51c}", // night light icon
            "Night Light",
            night_status,
            is_night_on,
            false,
            move |btn, sub_lbl, _| {
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
        let (capture_tile, _, _, _, _) = Self::create_feature_tile(
            "\u{e412}", // camera icon
            "Screen capture",
            "",
            false,
            false,
            |_btn, _lbl, _| {
                let _ = Command::new("niri").args(["msg", "action", "screenshot"]).spawn();
            },
            || {},
        );
        grid.attach(&capture_tile, 1, 1, 1, 1);

        // 6. Power Profile Tile (Col 2, Row 1 - Replaces Cast)
        let curr_power = PowerProfileService::get_profile();
        let (power_tile, power_btn, _power_title, power_sub, power_icon) = Self::create_feature_tile(
            curr_power.icon_code(),
            "Power mode",
            curr_power.display_name(),
            curr_power != PowerProfile::Balanced,
            false,
            move |btn, sub_lbl, icon_lbl| {
                let next = PowerProfileService::cycle_profile();
                if let Some(lbl) = sub_lbl {
                    lbl.set_text(next.display_name());
                }
                if let Some(icon) = icon_lbl {
                    icon.set_text(next.icon_code());
                }
                if next != PowerProfile::Balanced {
                    btn.add_css_class("active");
                } else {
                    btn.remove_css_class("active");
                }
            },
            || {},
        );
        grid.attach(&power_tile, 2, 1, 1, 1);

        Self {
            container: grid,
            wifi_btn,
            wifi_title,
            wifi_sub: wifi_sub.unwrap_or_else(|| Label::new(None)),
            bt_btn,
            bt_sub: bt_sub.unwrap_or_else(|| Label::new(None)),
            night_btn,
            night_sub: night_sub.unwrap_or_else(|| Label::new(None)),
            power_btn,
            power_icon,
            power_sub: power_sub.unwrap_or_else(|| Label::new(None)),
        }
    }

    /// Refresh live state asynchronously in background worker thread to ensure 0ms popup presentation
    pub fn async_refresh(grid_rc: Rc<Self>) {
        let (sender, receiver) = mpsc::channel::<(crate::services::network::NetworkState, bool, bool, PowerProfile)>();
        let (rfd, wfd) = crate::bar::create_event_pipe();

        thread::spawn(move || {
            let net_state = NetworkService::get_state();
            let bt_on = BluetoothService::is_bluetooth_enabled();
            let night_on = NightLightService::is_enabled();
            let power_prof = PowerProfileService::get_profile();

            let _ = sender.send((net_state, bt_on, night_on, power_prof));
            crate::bar::notify_pipe(wfd);
            unsafe { libc::close(wfd); }
        });

        glib::unix_fd_add_local(rfd, glib::IOCondition::IN, move |fd, _| {
            crate::bar::drain_pipe(fd);
            if let Ok((net_state, bt_on, night_on, power_prof)) = receiver.try_recv() {
                // Wi-Fi Tile synchronized with D-Bus
                if net_state.is_enabled {
                    grid_rc.wifi_btn.add_css_class("active");
                    if let Some(ssid) = &net_state.ssid {
                        grid_rc.wifi_title.set_text(&ssid);
                        grid_rc.wifi_sub.set_text("Connected");
                    } else {
                        grid_rc.wifi_title.set_text("Wi-Fi");
                        grid_rc.wifi_sub.set_text("Disconnected");
                    }
                } else {
                    grid_rc.wifi_btn.remove_css_class("active");
                    grid_rc.wifi_title.set_text("Wi-Fi");
                    grid_rc.wifi_sub.set_text("Off");
                }

                // Bluetooth Tile
                if bt_on {
                    grid_rc.bt_btn.add_css_class("active");
                    grid_rc.bt_sub.set_text("On");
                } else {
                    grid_rc.bt_btn.remove_css_class("active");
                    grid_rc.bt_sub.set_text("Off");
                }

                // Night Light Tile
                if night_on {
                    grid_rc.night_btn.add_css_class("active");
                    grid_rc.night_sub.set_text("On");
                } else {
                    grid_rc.night_btn.remove_css_class("active");
                    grid_rc.night_sub.set_text("Off");
                }

                // Power Profile Tile
                grid_rc.power_icon.set_text(power_prof.icon_code());
                grid_rc.power_sub.set_text(power_prof.display_name());
                if power_prof != PowerProfile::Balanced {
                    grid_rc.power_btn.add_css_class("active");
                } else {
                    grid_rc.power_btn.remove_css_class("active");
                }
            }
            unsafe { libc::close(fd); }
            glib::ControlFlow::Break
        });
    }

    fn create_feature_tile<FToggle, FArrow>(
        icon_code: &str,
        title: &str,
        sub_status: &str,
        is_active: bool,
        has_arrow: bool,
        on_toggle: FToggle,
        on_arrow: FArrow,
    ) -> (GtkBox, Button, Label, Option<Label>, Label)
    where
        FToggle: Fn(&Button, &Option<Label>, &Option<Label>) + 'static,
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

        // Wire click listener with circle_btn, sub_label, and icon references
        let btn_clone = circle_btn.clone();
        let sub_lbl_clone = sub_label.clone();
        let icon_clone = icon.clone();
        circle_btn.connect_clicked(move |_| {
            on_toggle(&btn_clone, &sub_lbl_clone, &Some(icon_clone.clone()));
        });

        (tile_box, circle_btn, title_label, sub_label, icon)
    }
}
