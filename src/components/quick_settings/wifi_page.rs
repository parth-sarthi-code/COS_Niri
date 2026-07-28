use crate::services::network::{NetworkService, WifiNetwork};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, GestureClick, Label, Orientation, PasswordEntry, ScrolledWindow, Switch};
use std::sync::mpsc;
use std::thread;

pub struct WifiPage {
    pub container: GtkBox,
}

impl WifiPage {
    pub fn new<FBack>(on_back: FBack) -> Self
    where
        FBack: Fn() + 'static,
    {
        let container = GtkBox::new(Orientation::Vertical, 8);
        container.add_css_class("qs-subpage");

        // Header: Back Arrow + "Network" Title + Power Switch
        let header = GtkBox::new(Orientation::Horizontal, 8);
        header.add_css_class("qs-subpage-header");

        let back_btn = Button::new();
        back_btn.add_css_class("qs-header-icon-btn");
        let back_icon = Label::new(Some("\u{e5c4}")); // arrow_back
        back_icon.add_css_class("ms-icon");
        back_btn.set_child(Some(&back_icon));
        back_btn.connect_clicked(move |_| {
            on_back();
        });
        header.append(&back_btn);

        let title = Label::new(Some("Network"));
        title.add_css_class("qs-subpage-title");
        title.set_hexpand(true);
        title.set_halign(gtk4::Align::Start);
        header.append(&title);

        let is_on = NetworkService::is_wifi_enabled();
        let toggle_switch = Switch::new();
        toggle_switch.set_active(is_on);

        header.append(&toggle_switch);
        container.append(&header);

        // Network List inside ScrolledWindow
        let scrolled = ScrolledWindow::new();
        scrolled.set_min_content_height(220);
        scrolled.set_max_content_height(260);
        scrolled.add_css_class("qs-subpage-scroll");

        let list_box = GtkBox::new(Orientation::Vertical, 4);
        list_box.add_css_class("qs-subpage-list");

        // Initial render
        Self::refresh_list(&list_box, is_on);

        // Switch toggle handler
        let list_box_clone = list_box.clone();
        toggle_switch.connect_state_set(move |_, state| {
            NetworkService::set_wifi_enabled(state);
            Self::refresh_list(&list_box_clone, state);
            glib::Propagation::Proceed
        });

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self { container }
    }

    fn refresh_list(list_box: &GtkBox, is_on: bool) {
        // Clear all list items
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        if !is_on {
            let off_lbl = Label::new(Some("Wi-Fi is turned off"));
            off_lbl.add_css_class("qs-subpage-empty");
            list_box.append(&off_lbl);
            return;
        }

        let loading_lbl = Label::new(Some("Scanning networks..."));
        loading_lbl.add_css_class("qs-subpage-empty");
        list_box.append(&loading_lbl);

        let (sender, receiver) = mpsc::channel::<Vec<WifiNetwork>>();
        let list_box_c = list_box.clone();

        thread::spawn(move || {
            let networks = NetworkService::scan_networks();
            let _ = sender.send(networks);
        });

        glib::idle_add_local(move || {
            if let Ok(networks) = receiver.try_recv() {
                // Clear loading label
                while let Some(child) = list_box_c.first_child() {
                    list_box_c.remove(&child);
                }

                if networks.is_empty() {
                    let empty_lbl = Label::new(Some("No networks found"));
                    empty_lbl.add_css_class("qs-subpage-empty");
                    list_box_c.append(&empty_lbl);
                } else {
                    for net in networks {
                        let item_container = GtkBox::new(Orientation::Vertical, 4);
                        item_container.add_css_class("qs-wifi-item-card");

                        let row = GtkBox::new(Orientation::Horizontal, 8);
                        row.add_css_class("qs-list-item-row");
                        row.set_valign(gtk4::Align::Center);

                        // Icon code based on signal
                        let icon_code = match net.signal {
                            75..=100 => "\u{e1d8}",
                            50..=74 => "\u{ebe1}",
                            25..=49 => "\u{ebd6}",
                            _ => "\u{ebe4}",
                        };

                        let icon = Label::new(Some(icon_code));
                        icon.add_css_class("ms-icon");
                        row.append(&icon);

                        let ssid_lbl = Label::new(Some(&net.ssid));
                        ssid_lbl.add_css_class("qs-list-title");
                        ssid_lbl.set_hexpand(true);
                        ssid_lbl.set_halign(gtk4::Align::Start);
                        row.append(&ssid_lbl);

                        if net.is_connected {
                            let conn_lbl = Label::new(Some("Connected"));
                            conn_lbl.add_css_class("qs-list-connected");
                            row.append(&conn_lbl);

                            // Disconnect button
                            let disc_btn = Button::with_label("Disconnect");
                            disc_btn.add_css_class("qs-disc-btn");
                            disc_btn.set_valign(gtk4::Align::Center);

                            let ssid_disc = net.ssid.clone();
                            let lb_ref = list_box_c.clone();
                            disc_btn.connect_clicked(move |_| {
                                NetworkService::disconnect_network(Some(&ssid_disc));
                                let lb_c = lb_ref.clone();
                                glib::timeout_add_seconds_local(1, move || {
                                    Self::refresh_list(&lb_c, true);
                                    glib::ControlFlow::Break
                                });
                            });
                            row.append(&disc_btn);
                        }

                        item_container.append(&row);

                        // If not connected, add expandable password prompt
                        if !net.is_connected {
                            let pass_box = GtkBox::new(Orientation::Horizontal, 6);
                            pass_box.add_css_class("qs-pass-box");
                            pass_box.set_visible(false);

                            let pass_entry = PasswordEntry::new();
                            pass_entry.add_css_class("qs-pass-input");
                            pass_entry.set_placeholder_text(Some("Password"));
                            pass_entry.set_hexpand(true);
                            pass_box.append(&pass_entry);

                            let connect_btn = Button::with_label("Connect");
                            connect_btn.add_css_class("qs-disc-btn");

                            let ssid_conn = net.ssid.clone();
                            let lb_ref2 = list_box_c.clone();
                            let p_entry = pass_entry.clone();

                            connect_btn.connect_clicked(move |_| {
                                let pass_text = p_entry.text().to_string();
                                let pass_opt = if pass_text.is_empty() {
                                    None
                                } else {
                                    Some(pass_text.as_str())
                                };
                                NetworkService::connect_network(&ssid_conn, pass_opt);
                                let lb_c = lb_ref2.clone();
                                glib::timeout_add_seconds_local(2, move || {
                                    Self::refresh_list(&lb_c, true);
                                    glib::ControlFlow::Break
                                });
                            });

                            pass_box.append(&connect_btn);
                            item_container.append(&pass_box);

                            // Toggle password box when clicking the row
                            let gesture = GestureClick::new();
                            let p_box_toggle = pass_box.clone();
                            gesture.connect_pressed(move |_, _, _, _| {
                                let is_vis = p_box_toggle.is_visible();
                                p_box_toggle.set_visible(!is_vis);
                            });
                            row.add_controller(gesture);
                        }

                        list_box_c.append(&item_container);
                    }
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
}
