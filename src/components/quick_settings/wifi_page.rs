use crate::services::network::{NetworkService, WifiNetwork};
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, ScrolledWindow, Switch};
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
        toggle_switch.connect_state_set(|_, state| {
            NetworkService::set_wifi_enabled(state);
            glib::Propagation::Proceed
        });
        header.append(&toggle_switch);

        container.append(&header);

        // Network List inside ScrolledWindow
        let scrolled = ScrolledWindow::new();
        scrolled.set_min_content_height(220);
        scrolled.set_max_content_height(260);
        scrolled.add_css_class("qs-subpage-scroll");

        let list_box = GtkBox::new(Orientation::Vertical, 4);
        list_box.add_css_class("qs-subpage-list");

        if is_on {
            let loading_lbl = Label::new(Some("Scanning networks..."));
            loading_lbl.add_css_class("qs-subpage-empty");
            list_box.append(&loading_lbl);

            let (sender, receiver) = mpsc::channel::<Vec<WifiNetwork>>();
            let list_box_clone = list_box.clone();

            thread::spawn(move || {
                let networks = NetworkService::scan_networks();
                let _ = sender.send(networks);
            });

            glib::idle_add_local(move || {
                if let Ok(networks) = receiver.try_recv() {
                    while let Some(child) = list_box_clone.first_child() {
                        list_box_clone.remove(&child);
                    }

                    if networks.is_empty() {
                        let empty_lbl = Label::new(Some("No networks found"));
                        empty_lbl.add_css_class("qs-subpage-empty");
                        list_box_clone.append(&empty_lbl);
                    } else {
                        for net in networks {
                            let item_btn = Button::new();
                            item_btn.add_css_class("qs-list-item");

                            let row = GtkBox::new(Orientation::Horizontal, 8);
                            let icon = Label::new(Some("\u{e63e}")); // wifi icon
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
                            }

                            item_btn.set_child(Some(&row));

                            let ssid_clone = net.ssid.clone();
                            item_btn.connect_clicked(move |_| {
                                NetworkService::connect_network(&ssid_clone, None);
                            });

                            list_box_clone.append(&item_btn);
                        }
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        } else {
            let off_lbl = Label::new(Some("Wi-Fi is turned off"));
            off_lbl.add_css_class("qs-subpage-empty");
            list_box.append(&off_lbl);
        }

        scrolled.set_child(Some(&list_box));
        container.append(&scrolled);

        Self { container }
    }
}
